use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, Position, Size,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub const AVATAR_WINDOW_LABEL: &str = "avatar";
pub const AVATAR_STATE_EVENT: &str = "avatar-state-changed";
pub const AVATAR_BUBBLE_EVENT: &str = "avatar-bubble";

const AVATAR_SCALE_MIN: f64 = 0.7;
const AVATAR_SCALE_MAX: f64 = 1.3;
const AVATAR_SCALE_DEFAULT: f64 = 0.9;
const AVATAR_OPACITY_DEFAULT: f64 = 0.82;
const AVATAR_WINDOW_BASE_WIDTH: f64 = 152.0;
const AVATAR_WINDOW_BASE_HEIGHT: f64 = 170.0;
const AVATAR_WINDOW_WIDTH: f64 = AVATAR_WINDOW_BASE_WIDTH * AVATAR_SCALE_DEFAULT;
const AVATAR_WINDOW_HEIGHT: f64 = AVATAR_WINDOW_BASE_HEIGHT * AVATAR_SCALE_DEFAULT;
const AVATAR_WINDOW_MARGIN: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AvatarStatePayload {
    pub mode: String,
    pub app_name: String,
    pub context_label: String,
    pub hint: String,
    pub is_idle: bool,
    pub is_generating_report: bool,
    pub avatar_opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AvatarBubblePayload {
    pub message: String,
    pub tone: String,
}

impl AvatarBubblePayload {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: "info".to_string(),
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: "success".to_string(),
        }
    }
}

pub fn default_avatar_state() -> AvatarStatePayload {
    AvatarStatePayload {
        mode: "idle".to_string(),
        app_name: "Work Review".to_string(),
        context_label: "待命中".to_string(),
        hint: "准备陪你开始工作".to_string(),
        is_idle: true,
        is_generating_report: false,
        avatar_opacity: AVATAR_OPACITY_DEFAULT,
    }
}

pub fn derive_avatar_state(
    app_name: &str,
    window_title: &str,
    browser_url: Option<&str>,
    is_idle: bool,
    is_generating_report: bool,
) -> AvatarStatePayload {
    derive_avatar_state_with_rules(
        &[],
        app_name,
        window_title,
        browser_url,
        is_idle,
        is_generating_report,
    )
}

pub fn derive_avatar_state_with_rules(
    rules: &[crate::config::AppCategoryRule],
    app_name: &str,
    window_title: &str,
    browser_url: Option<&str>,
    is_idle: bool,
    is_generating_report: bool,
) -> AvatarStatePayload {
    let app_name = normalize_app_name(app_name);
    let title_lower = window_title.trim().to_lowercase();
    let url_lower = browser_url.unwrap_or_default().trim().to_lowercase();
    let manual_base_category = crate::monitor::find_category_override(rules, app_name.as_str());
    let base_category =
        crate::monitor::categorize_app_with_rules(rules, app_name.as_str(), window_title);
    let classification = crate::activity_classifier::classify_activity_with_base_category(
        app_name.as_str(),
        window_title,
        browser_url,
        &base_category,
    );

    if is_generating_report {
        return AvatarStatePayload {
            mode: "generating".to_string(),
            app_name,
            context_label: "生成中".to_string(),
            hint: "正在帮你整理今天的内容".to_string(),
            is_idle,
            is_generating_report: true,
            avatar_opacity: AVATAR_OPACITY_DEFAULT,
        };
    }

    if is_idle {
        return AvatarStatePayload {
            mode: "idle".to_string(),
            app_name,
            context_label: "待机中".to_string(),
            hint: "先歇一会，恢复状态再继续".to_string(),
            is_idle: true,
            is_generating_report: false,
            avatar_opacity: AVATAR_OPACITY_DEFAULT,
        };
    }

    if let Some(base_category) = manual_base_category.as_deref() {
        return match base_category {
            "development" => avatar_state(
                "working",
                app_name,
                "编码中",
                "先把这一段逻辑收住，再看下一处。",
            ),
            "browser" => {
                let (context_label, hint) = if contains_any(
                    &title_lower,
                    &["文档", "readme", "docs", "guide", "manual", "wiki"],
                ) || contains_any(
                    &url_lower,
                    &["/docs", "readme", "wiki", "developer.", "docs."],
                ) {
                    ("文档中", "先把文档结构看清，再动手修改。")
                } else {
                    ("阅读中", "正在吸收内容，先别打断节奏")
                };

                avatar_state("reading", app_name, context_label, hint)
            }
            "communication" => avatar_state(
                "working",
                app_name,
                "沟通中",
                "先把来回沟通收住，再继续推进。",
            ),
            "office" => avatar_state(
                "working",
                app_name,
                "办公中",
                "先把这一段事务处理完，再切换上下文。",
            ),
            "design" => avatar_state(
                "working",
                app_name,
                "创作中",
                "先把这一版想法落下来，再比较取舍。",
            ),
            "entertainment" => avatar_state(
                "slacking",
                app_name,
                "休息中",
                "放松可以，但别把节奏彻底丢掉。",
            ),
            _ => avatar_state("working", app_name, "办公中", "保持推进，先把这一段收住"),
        };
    }

    match classification.semantic_category.as_str() {
        "会议沟通" => {
            let (context_label, hint) = if contains_any(
                &title_lower,
                &["共享屏幕", "演示", "demo", "汇报", "review"],
            ) {
                ("演示中", "先把演示节奏收稳，再继续推进。")
            } else if contains_any(&title_lower, &["语音通话", "call", "huddle"]) {
                ("通话中", "先把关键结论对齐，再回到执行。")
            } else {
                ("开会中", "先把沟通结论收拢，再继续推进。")
            };

            avatar_state("meeting", app_name, context_label, hint)
        }
        "音乐音频" => {
            let (context_label, hint) = if contains_any(&title_lower, &["播客", "podcast"]) {
                ("播客中", "先把这段观点听完，再决定要不要切走。")
            } else {
                ("听歌中", "保持节奏，但别让注意力被带跑。")
            };

            avatar_state("music", app_name, context_label, hint)
        }
        "视频内容" => {
            let (context_label, hint) = if contains_any(
                &title_lower,
                &["课程", "教程", "lesson", "training", "回放"],
            ) {
                ("学习中", "先吸收这一段，再整理下一步动作。")
            } else if contains_any(&title_lower, &["直播", "live"]) {
                ("直播中", "先看清实时信息，再判断是否介入。")
            } else {
                ("视频中", "先看完这一段，再决定下一步。")
            };

            avatar_state("video", app_name, context_label, hint)
        }
        "资料阅读" => {
            let (context_label, hint) = if contains_any(
                &title_lower,
                &["文档", "readme", "docs", "guide", "manual", "wiki"],
            ) || contains_any(
                &url_lower,
                &["/docs", "readme", "wiki", "developer.", "docs."],
            ) {
                ("文档中", "先把文档结构看清，再动手修改。")
            } else {
                ("阅读中", "正在吸收内容，先别打断节奏")
            };

            avatar_state("reading", app_name, context_label, hint)
        }
        "资料调研" => avatar_state(
            "reading",
            app_name,
            "调研中",
            "先收拢信息，再决定怎么推进。",
        ),
        "休息娱乐" => avatar_state(
            "slacking",
            app_name,
            "休息中",
            "放松可以，但别把节奏彻底丢掉。",
        ),
        "即时聊天" => avatar_state(
            "working",
            app_name,
            "沟通中",
            "先把来回沟通收住，再继续推进。",
        ),
        "内容撰写" => {
            let (context_label, hint) =
                if contains_any(&title_lower, &["日报", "周报", "复盘", "总结"]) {
                    ("总结中", "先把结论写完整，再回头补细节。")
                } else if contains_any(&title_lower, &["方案", "需求", "prd"]) {
                    ("方案中", "先把核心方案收拢，再继续展开。")
                } else {
                    ("写作中", "先把这段内容写完整，再回头修。")
                };

            avatar_state("working", app_name, context_label, hint)
        }
        "任务规划" => {
            let (context_label, hint) =
                if contains_any(
                    &title_lower,
                    &["排期", "里程碑", "backlog", "sprint", "roadmap"],
                ) || contains_any(&url_lower, &["linear.app", "jira", "atlassian.net"])
                {
                    ("排期中", "先把节奏和先后顺序排清楚。")
                } else if contains_any(&title_lower, &["待办", "任务", "看板", "board"]) {
                    ("拆解中", "先把任务拆细，再进入执行。")
                } else {
                    ("规划中", "先把优先级排清楚，再进入执行。")
                };

            avatar_state("working", app_name, context_label, hint)
        }
        "编码开发" => avatar_state(
            "working",
            app_name,
            "编码中",
            "先把这一段逻辑收住，再看下一处。",
        ),
        "设计创作" => avatar_state(
            "working",
            app_name,
            "创作中",
            "先把这一版想法落下来，再比较取舍。",
        ),
        "未知活动" if classification.confidence < 60 => avatar_state(
            "working",
            app_name,
            "判断中",
            "信息还不够明确，先继续观察一下。",
        ),
        _ => avatar_state("working", app_name, "办公中", "保持推进，先把这一段收住"),
    }
}

pub fn apply_avatar_opacity(mut payload: AvatarStatePayload, opacity: f64) -> AvatarStatePayload {
    payload.avatar_opacity = opacity;
    payload
}

pub fn sync_avatar_window(
    app: &AppHandle,
    enabled: bool,
    scale: f64,
    saved_position: Option<(i32, i32)>,
) -> tauri::Result<()> {
    if enabled {
        ensure_avatar_window(app, scale)?;
        if let Some(window) = app.get_webview_window(AVATAR_WINDOW_LABEL) {
            let normalized_scale = normalize_avatar_scale(scale);
            let (x, y) = default_avatar_position(app, normalized_scale, saved_position);
            resize_avatar_window(&window, normalized_scale);
            let _ = window.set_always_on_top(true);
            let _ = window.set_visible_on_all_workspaces(true);
            let _ = window.set_skip_taskbar(true);
            let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
            let _ = window.unminimize();
            let _ = window.show();
        }
    } else if let Some(window) = app.get_webview_window(AVATAR_WINDOW_LABEL) {
        let _ = window.hide();
    }

    Ok(())
}

pub fn emit_avatar_state(app: &AppHandle, payload: &AvatarStatePayload) {
    if let Some(window) = app.get_webview_window(AVATAR_WINDOW_LABEL) {
        let _ = window.emit(AVATAR_STATE_EVENT, payload);
    }
}

pub fn emit_avatar_bubble(app: &AppHandle, payload: &AvatarBubblePayload) {
    if let Some(window) = app.get_webview_window(AVATAR_WINDOW_LABEL) {
        let _ = window.emit(AVATAR_BUBBLE_EVENT, payload);
    }
}

fn ensure_avatar_window(app: &AppHandle, scale: f64) -> tauri::Result<()> {
    if app.get_webview_window(AVATAR_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let (window_width, window_height) = avatar_window_size(scale);

    let window = WebviewWindowBuilder::new(app, AVATAR_WINDOW_LABEL, WebviewUrl::default())
        .title("Work Review Avatar")
        .inner_size(window_width, window_height)
        .min_inner_size(window_width, window_height)
        .max_inner_size(window_width, window_height)
        .position(40.0, 40.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .decorations(false)
        .transparent(true)
        .visible(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .build()?;

    let _ = window.set_always_on_top(true);
    let _ = window.set_visible_on_all_workspaces(true);
    let _ = window.set_skip_taskbar(true);

    Ok(())
}

fn resize_avatar_window(window: &WebviewWindow, scale: f64) {
    let (window_width, window_height) = avatar_window_size(scale);
    let _ = window.set_size(Size::Logical(LogicalSize::new(window_width, window_height)));
    let _ = window.set_min_size(Some(Size::Logical(LogicalSize::new(
        window_width,
        window_height,
    ))));
    let _ = window.set_max_size(Some(Size::Logical(LogicalSize::new(
        window_width,
        window_height,
    ))));
}

fn default_avatar_position(
    app: &AppHandle,
    scale: f64,
    saved_position: Option<(i32, i32)>,
) -> (i32, i32) {
    if let Some(main_window) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = main_window.current_monitor() {
            let anchor = match (main_window.outer_position(), main_window.outer_size()) {
                (Ok(position), Ok(size)) => Some(Rect {
                    x: position.x,
                    y: position.y,
                    width: size.width as i32,
                    height: size.height as i32,
                }),
                _ => None,
            };

            return compute_avatar_position(monitor, anchor, saved_position, scale);
        }
    }

    if let Ok(Some(monitor)) = app.primary_monitor() {
        return compute_avatar_position(monitor, None, saved_position, scale);
    }

    saved_position.unwrap_or((40, 40))
}

fn compute_avatar_position(
    monitor: Monitor,
    anchor: Option<Rect>,
    saved_position: Option<(i32, i32)>,
    scale: f64,
) -> (i32, i32) {
    let work_area = monitor.work_area();
    let bounds = Rect {
        x: work_area.position.x,
        y: work_area.position.y,
        width: work_area.size.width as i32,
        height: work_area.size.height as i32,
    };

    resolve_avatar_position(bounds, anchor, saved_position, scale)
}

fn resolve_avatar_position(
    bounds: Rect,
    anchor: Option<Rect>,
    saved_position: Option<(i32, i32)>,
    scale: f64,
) -> (i32, i32) {
    let (window_width, window_height) = avatar_window_size(scale);

    if let Some((saved_x, saved_y)) = saved_position {
        return clamp_avatar_position_with_size(
            bounds,
            saved_x,
            saved_y,
            window_width,
            window_height,
        );
    }

    let preferred_x = anchor
        .map(|rect| rect.x + rect.width - window_width as i32 - AVATAR_WINDOW_MARGIN as i32)
        .unwrap_or(bounds.x + bounds.width - window_width as i32 - AVATAR_WINDOW_MARGIN as i32);
    let preferred_y = anchor
        .map(|rect| rect.y + rect.height - window_height as i32 - AVATAR_WINDOW_MARGIN as i32)
        .unwrap_or(bounds.y + bounds.height - window_height as i32 - AVATAR_WINDOW_MARGIN as i32);

    clamp_avatar_position_with_size(
        bounds,
        preferred_x,
        preferred_y,
        window_width,
        window_height,
    )
}

fn clamp_avatar_position(bounds: Rect, preferred_x: i32, preferred_y: i32) -> (i32, i32) {
    clamp_avatar_position_with_size(
        bounds,
        preferred_x,
        preferred_y,
        AVATAR_WINDOW_WIDTH,
        AVATAR_WINDOW_HEIGHT,
    )
}

fn clamp_avatar_position_with_size(
    bounds: Rect,
    preferred_x: i32,
    preferred_y: i32,
    window_width: f64,
    window_height: f64,
) -> (i32, i32) {
    let max_x = (bounds.x + bounds.width - window_width as i32).max(bounds.x);
    let max_y = (bounds.y + bounds.height - window_height as i32).max(bounds.y);

    (
        preferred_x.clamp(bounds.x, max_x),
        preferred_y.clamp(bounds.y, max_y),
    )
}

fn avatar_window_size(scale: f64) -> (f64, f64) {
    let normalized_scale = normalize_avatar_scale(scale);
    (
        ((AVATAR_WINDOW_BASE_WIDTH * normalized_scale) * 10.0).round() / 10.0,
        ((AVATAR_WINDOW_BASE_HEIGHT * normalized_scale) * 10.0).round() / 10.0,
    )
}

fn normalize_avatar_scale(scale: f64) -> f64 {
    if !scale.is_finite() {
        return AVATAR_SCALE_DEFAULT;
    }

    scale.clamp(AVATAR_SCALE_MIN, AVATAR_SCALE_MAX)
}

fn normalize_app_name(app_name: &str) -> String {
    let trimmed = app_name.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
        "当前任务".to_string()
    } else {
        trimmed.to_string()
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn avatar_state(
    mode: &str,
    app_name: String,
    context_label: &str,
    hint: &str,
) -> AvatarStatePayload {
    AvatarStatePayload {
        mode: mode.to_string(),
        app_name,
        context_label: context_label.to_string(),
        hint: hint.to_string(),
        is_idle: false,
        is_generating_report: false,
        avatar_opacity: AVATAR_OPACITY_DEFAULT,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        avatar_window_size, clamp_avatar_position, default_avatar_state, derive_avatar_state,
        derive_avatar_state_with_rules, resolve_avatar_position, Rect, AVATAR_WINDOW_HEIGHT,
        AVATAR_WINDOW_WIDTH,
    };
    use crate::config::AppCategoryRule;

    #[test]
    fn 空闲状态应优先进入待机模式() {
        let state = derive_avatar_state("VS Code", "main.rs", None, true, false);

        assert_eq!(state.mode, "idle");
        assert!(state.is_idle);
        assert_eq!(state.context_label, "待机中");
    }

    #[test]
    fn 日报生成应优先进入生成模式() {
        let state = derive_avatar_state("Google Chrome", "日报", None, false, true);

        assert_eq!(state.mode, "generating");
        assert!(state.is_generating_report);
        assert_eq!(state.context_label, "生成中");
    }

    #[test]
    fn 浏览器上下文应识别为阅读模式() {
        let state = derive_avatar_state("Google Chrome", "产品文档 - docs", None, false, false);

        assert_eq!(state.mode, "reading");
        assert_eq!(state.context_label, "文档中");
    }

    #[test]
    fn 会议应用应识别为开会状态() {
        let state = derive_avatar_state("Zoom", "项目例会", None, false, false);

        assert_eq!(state.mode, "meeting");
        assert_eq!(state.context_label, "开会中");
    }

    #[test]
    fn 普通沟通工具不应直接识别为开会状态() {
        let state = derive_avatar_state("Slack", "设计评审结论整理", None, false, false);

        assert_ne!(state.mode, "meeting");
        assert_eq!(state.mode, "working");
        assert_eq!(state.context_label, "沟通中");
    }

    #[test]
    fn 音乐应用应识别为音乐状态() {
        let state = derive_avatar_state("Spotify", "Daily Mix", None, false, false);

        assert_eq!(state.mode, "music");
        assert_eq!(state.context_label, "听歌中");
    }

    #[test]
    fn 视频应用应识别为视频状态() {
        let state = derive_avatar_state("Bilibili", "RustConf 回放", None, false, false);

        assert_eq!(state.mode, "video");
        assert_eq!(state.context_label, "学习中");
    }

    #[test]
    fn 娱乐场景应识别为摸鱼状态() {
        let state = derive_avatar_state("Steam", "Balatro", None, false, false);

        assert_eq!(state.mode, "slacking");
        assert_eq!(state.context_label, "休息中");
    }

    #[test]
    fn 非阅读上下文应识别为工作模式() {
        let state = derive_avatar_state("Cursor", "commands.rs", None, false, false);

        assert_eq!(state.mode, "working");
        assert_eq!(state.context_label, "编码中");
    }

    #[test]
    fn github_拉取请求页面应识别为编码中() {
        let state = derive_avatar_state(
            "Google Chrome",
            "Fix updater retry · Pull Request #12 · wm94i/Work_Review",
            Some("https://github.com/wm94i/Work_Review/pull/12"),
            false,
            false,
        );

        assert_eq!(state.mode, "working");
        assert_eq!(state.context_label, "编码中");
    }

    #[test]
    fn 浏览器任务看板应识别为排期中() {
        let state = derive_avatar_state(
            "Google Chrome",
            "Sprint 15 Board - Linear",
            Some("https://linear.app/work-review/board"),
            false,
            false,
        );

        assert_eq!(state.mode, "working");
        assert_eq!(state.context_label, "排期中");
    }

    #[test]
    fn 共享屏幕会议应识别为演示中() {
        let state = derive_avatar_state("Zoom", "项目评审 - 共享屏幕", None, false, false);

        assert_eq!(state.mode, "meeting");
        assert_eq!(state.context_label, "演示中");
    }

    #[test]
    fn 教程视频应识别为学习中() {
        let state = derive_avatar_state("Bilibili", "Tauri 教程 第三课", None, false, false);

        assert_eq!(state.mode, "video");
        assert_eq!(state.context_label, "学习中");
    }

    #[test]
    fn 播客内容应识别为播客中() {
        let state = derive_avatar_state("Spotify", "AI Podcast 第 42 期", None, false, false);

        assert_eq!(state.mode, "music");
        assert_eq!(state.context_label, "播客中");
    }

    #[test]
    fn 低证据活动应在桌宠层回退为判断中() {
        let state = derive_avatar_state("UnknownApp", "首页", None, false, false);

        assert_eq!(state.mode, "working");
        assert_eq!(state.context_label, "判断中");
    }

    #[test]
    fn 手动分类规则应影响桌宠动作判断() {
        let state = derive_avatar_state_with_rules(
            &[AppCategoryRule {
                app_name: "Steam".to_string(),
                category: "design".to_string(),
            }],
            "Steam",
            "首页",
            None,
            false,
            false,
        );

        assert_eq!(state.mode, "working");
        assert_eq!(state.context_label, "创作中");
    }

    #[test]
    fn 默认状态应可直接用于初始化() {
        let state = default_avatar_state();

        assert_eq!(state.mode, "idle");
        assert_eq!(state.app_name, "Work Review");
        assert!(!state.is_generating_report);
    }

    #[test]
    fn 桌宠窗口尺寸应随缩放变化() {
        let (small_w, small_h) = avatar_window_size(0.7);
        let (default_w, default_h) = avatar_window_size(0.9);
        let (large_w, large_h) = avatar_window_size(1.3);

        assert!(small_w < default_w);
        assert!(small_h < default_h);
        assert!(large_w > default_w);
        assert!(large_h > default_h);
        assert_eq!((default_w, default_h), (136.8, 153.0));
    }

    #[test]
    fn 桌宠位置应优先吸附到锚点右下侧() {
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 1440,
            height: 900,
        };

        let (x, y) = clamp_avatar_position(bounds, 716, 356);

        assert_eq!((x, y), (716, 356));
    }

    #[test]
    fn 桌宠位置超出屏幕时应被钳制回可视区域() {
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
        };

        let (x, y) = clamp_avatar_position(bounds, 1200, 600);

        assert_eq!(
            (x, y),
            (
                1280 - AVATAR_WINDOW_WIDTH as i32,
                720 - AVATAR_WINDOW_HEIGHT as i32
            )
        );
    }

    #[test]
    fn 已保存桌宠位置应优先于默认吸附位置() {
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 1440,
            height: 900,
        };
        let anchor = Rect {
            x: 100,
            y: 100,
            width: 900,
            height: 600,
        };

        let (x, y) = resolve_avatar_position(bounds, Some(anchor), Some((120, 240)), 0.9);

        assert_eq!((x, y), (120, 240));
    }

    #[test]
    fn 已保存桌宠位置超出范围时应回到可视区域() {
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 1280,
            height: 720,
        };

        let (x, y) = resolve_avatar_position(bounds, None, Some((1600, 900)), 0.9);

        assert_eq!(
            (x, y),
            (
                1280 - AVATAR_WINDOW_WIDTH as i32,
                720 - AVATAR_WINDOW_HEIGHT as i32
            )
        );
    }
}
