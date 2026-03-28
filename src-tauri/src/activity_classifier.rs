use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityClassification {
    pub base_category: String,
    pub semantic_category: String,
    pub confidence: u8,
    pub evidence: Vec<String>,
}

pub fn classify_activity(
    app_name: &str,
    window_title: &str,
    browser_url: Option<&str>,
) -> ActivityClassification {
    let base_category = crate::monitor::categorize_app(app_name.trim(), window_title.trim());
    classify_activity_with_base_category(app_name, window_title, browser_url, &base_category)
}

pub fn classify_activity_with_base_category(
    app_name: &str,
    window_title: &str,
    browser_url: Option<&str>,
    base_category: &str,
) -> ActivityClassification {
    let normalized_app = app_name.trim();
    let normalized_title = window_title.trim();
    let app_lower = normalized_app.to_lowercase();
    let title_lower = normalized_title.to_lowercase();
    let url_lower = browser_url.unwrap_or_default().trim().to_lowercase();
    let base_category = crate::monitor::normalize_category_key(base_category);

    let mut scores: HashMap<&'static str, i32> = HashMap::new();
    let mut evidence: HashMap<&'static str, Vec<String>> = HashMap::new();

    let mut add = |label: &'static str, score: i32, reason: &str| {
        *scores.entry(label).or_insert(0) += score;
        evidence.entry(label).or_default().push(reason.to_string());
    };

    match base_category.as_str() {
        "development" => add("编码开发", 28, "基础类别命中开发工具"),
        "communication" => add("即时聊天", 16, "基础类别命中通讯协作"),
        "office" => add("内容撰写", 12, "基础类别命中办公软件"),
        "design" => add("设计创作", 24, "基础类别命中设计工具"),
        "browser" => add("资料阅读", 16, "基础类别命中浏览器"),
        "entertainment" => add("休息娱乐", 8, "基础类别命中娱乐"),
        _ => {}
    }

    if contains_any(
        &app_lower,
        &[
            "cursor",
            "code",
            "visual studio",
            "idea",
            "pycharm",
            "webstorm",
            "goland",
            "terminal",
            "powershell",
            "warp",
            "wezterm",
            "docker",
            "postman",
            "insomnia",
            "dbeaver",
        ],
    ) {
        add("编码开发", 48, "应用名命中编码开发工具");
    }

    if contains_any(
        &title_lower,
        &[
            ".rs",
            ".ts",
            ".tsx",
            ".js",
            ".jsx",
            ".vue",
            ".py",
            ".go",
            ".java",
            ".kt",
            ".sql",
            "cargo",
            "npm",
            "git",
            "commit",
            "pull request",
            "branch",
        ],
    ) {
        add("编码开发", 24, "窗口标题命中代码或工程信号");
    }

    if contains_any(
        &title_lower,
        &[
            "pull request",
            "code review",
            "review comments",
            "changed files",
            "diff",
        ],
    ) {
        add("编码开发", 22, "窗口标题命中代码评审信号");
    }

    if contains_any(
        &app_lower,
        &[
            "word",
            "typora",
            "obsidian",
            "notion",
            "飞书文档",
            "wps",
            "onenote",
        ],
    ) {
        add("内容撰写", 36, "应用名命中内容撰写工具");
    }

    if contains_any(
        &title_lower,
        &[
            "日报", "周报", "方案", "需求", "prd", "复盘", "邮件", "memo", "草稿",
        ],
    ) {
        add("内容撰写", 28, "窗口标题命中内容撰写词");
    }

    if contains_any(
        &app_lower,
        &[
            "preview", "reader", "pdf", "kindle", "zotero", "notion", "obsidian",
        ],
    ) {
        add("资料阅读", 24, "应用名命中资料阅读工具");
    }

    if contains_any(
        &title_lower,
        &[
            "文档", "readme", "docs", "guide", "manual", "wiki", "帮助", "教程",
        ],
    ) {
        add("资料阅读", 32, "窗口标题命中文档阅读词");
    }

    if contains_any(
        &url_lower,
        &[
            "docs.",
            "/docs",
            "readme",
            "wiki",
            "developer.",
            "tauri.app",
            "doc.rust-lang.org",
        ],
    ) {
        add("资料阅读", 42, "浏览器地址命中文档站");
    }

    if contains_any(
        &url_lower,
        &[
            "google.com/search",
            "bing.com/search",
            "baidu.com/s",
            "sogou.com/web",
            "so.com/s",
        ],
    ) {
        add("资料调研", 52, "浏览器地址命中搜索引擎结果页");
    }

    if contains_any(
        &title_lower,
        &[
            "搜索",
            "search",
            "对比",
            "最佳实践",
            "怎么实现",
            "如何",
            "方案选择",
        ],
    ) {
        add("资料调研", 26, "窗口标题命中资料调研词");
    }

    if contains_any(
        &url_lower,
        &[
            "stackoverflow.com",
            "github.com",
            "zhihu.com",
            "juejin.cn",
            "segmentfault.com",
        ],
    ) {
        add("资料调研", 12, "浏览器地址命中资料社区");
    }

    if contains_any(
        &app_lower,
        &["zoom", "teams", "meeting", "meet", "腾讯会议", "飞书会议"],
    ) {
        add("会议沟通", 64, "应用名命中会议工具");
    }

    if contains_any(
        &title_lower,
        &[
            "会议",
            "例会",
            "meeting",
            "call",
            "huddle",
            "共享屏幕",
            "语音通话",
            "视频会议",
        ],
    ) {
        add("会议沟通", 30, "窗口标题命中会议词");
    }

    if contains_any(
        &app_lower,
        &[
            "slack",
            "discord",
            "wechat",
            "微信",
            "wecom",
            "企业微信",
            "telegram",
            "qq",
            "飞书",
            "lark",
            "dingtalk",
            "钉钉",
        ],
    ) {
        add("即时聊天", 36, "应用名命中即时聊天工具");
    }

    if contains_any(
        &title_lower,
        &["群聊", "频道", "聊天", "消息", "联系人", "讨论", "讨论组"],
    ) {
        add("即时聊天", 18, "窗口标题命中聊天词");
    }

    if contains_any(
        &app_lower,
        &["jira", "linear", "trello", "asana", "clickup", "todoist"],
    ) {
        add("任务规划", 50, "应用名命中任务规划工具");
    }

    if contains_any(
        &title_lower,
        &[
            "待办",
            "迭代",
            "任务",
            "排期",
            "看板",
            "里程碑",
            "backlog",
            "sprint",
        ],
    ) {
        add("任务规划", 26, "窗口标题命中任务规划词");
    }

    if contains_any(
        &url_lower,
        &[
            "linear.app",
            "jira",
            "atlassian.net",
            "trello.com",
            "clickup.com",
            "asana.com",
        ],
    ) {
        add("任务规划", 46, "浏览器地址命中任务规划平台");
    }

    if contains_any(
        &app_lower,
        &[
            "figma",
            "sketch",
            "photoshop",
            "illustrator",
            "canva",
            "affinity",
        ],
    ) {
        add("设计创作", 56, "应用名命中设计创作工具");
    }

    if contains_any(&title_lower, &["画板", "组件", "原型", "设计稿", "frame"]) {
        add("设计创作", 20, "窗口标题命中设计创作词");
    }

    if contains_any(
        &app_lower,
        &[
            "spotify",
            "music",
            "网易云",
            "qqmusic",
            "apple music",
            "podcast",
        ],
    ) {
        add("音乐音频", 54, "应用名命中音乐音频工具");
    }

    if contains_any(
        &title_lower,
        &["歌单", "专辑", "歌词", "播客", "playlist", "album"],
    ) {
        add("音乐音频", 20, "窗口标题命中音乐音频词");
    }

    if contains_any(
        &app_lower,
        &[
            "youtube",
            "bilibili",
            "爱奇艺",
            "腾讯视频",
            "netflix",
            "vlc",
            "iina",
        ],
    ) {
        add("视频内容", 52, "应用名命中视频工具");
    }

    if contains_any(
        &title_lower,
        &["回放", "课程", "直播", "视频", "episode", "movie", "播放"],
    ) {
        add("视频内容", 24, "窗口标题命中视频内容词");
    }

    if contains_any(
        &app_lower,
        &[
            "steam",
            "weibo",
            "微博",
            "douyin",
            "抖音",
            "小红书",
            "rednote",
            "reddit",
        ],
    ) {
        add("休息娱乐", 46, "应用名命中休息娱乐工具");
    }

    if app_lower.contains("steam") {
        add("休息娱乐", 18, "Steam 前台通常表示休息娱乐");
    }

    if contains_any(
        &title_lower,
        &["游戏", "动态", "社区", "直播", "刷视频", "推荐"],
    ) {
        add("休息娱乐", 20, "窗口标题命中休息娱乐词");
    }

    if url_lower.contains("github.com/") {
        add("编码开发", 12, "浏览器地址命中代码托管站");
        add("资料调研", 8, "浏览器地址命中工程资料站");
    }

    if contains_any(
        &url_lower,
        &[
            "github.com/",
            "/pull/",
            "/compare/",
            "/commit/",
            "/blob/",
            "/tree/",
            "/files",
        ],
    ) && !contains_any(&url_lower, &["/wiki", "/releases", "/discussions"])
    {
        add("编码开发", 28, "浏览器地址命中代码评审或仓库文件页");
    }

    let mut ranked: Vec<(&'static str, i32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

    let Some((top_label, top_score)) = ranked.first().copied() else {
        return ActivityClassification {
            base_category,
            semantic_category: "未知活动".to_string(),
            confidence: 20,
            evidence: vec!["未命中任何有效分类信号".to_string()],
        };
    };

    let second_score = ranked.get(1).map(|(_, score)| *score).unwrap_or_default();
    let score_gap = top_score - second_score;
    let threshold = semantic_threshold(top_label);
    let high_risk = matches!(top_label, "会议沟通" | "休息娱乐");

    if top_score < threshold || (high_risk && score_gap < 10) {
        return ActivityClassification {
            base_category,
            semantic_category: "未知活动".to_string(),
            confidence: 45,
            evidence: vec![format!(
                "最高分 {top_score} 未达到分类阈值 {threshold}，或与次高分过近"
            )],
        };
    }

    let confidence = ((top_score + score_gap).clamp(55, 98)) as u8;

    ActivityClassification {
        base_category: base_category.to_string(),
        semantic_category: top_label.to_string(),
        confidence,
        evidence: evidence.remove(top_label).unwrap_or_default(),
    }
}

fn semantic_threshold(label: &str) -> i32 {
    match label {
        "会议沟通" | "休息娱乐" => 60,
        "即时聊天" | "视频内容" => 50,
        "编码开发" | "内容撰写" | "资料阅读" | "资料调研" => 40,
        "任务规划" | "设计创作" | "音乐音频" => 42,
        _ => 45,
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::classify_activity;

    #[test]
    fn 普通聊天工具应识别为即时聊天() {
        let result = classify_activity("Slack", "产品方案讨论", None);

        assert_eq!(result.base_category, "communication");
        assert_eq!(result.semantic_category, "即时聊天");
        assert!(result.confidence >= 70);
    }

    #[test]
    fn 会议工具应识别为会议沟通() {
        let result = classify_activity("Zoom", "项目例会", None);

        assert_eq!(result.semantic_category, "会议沟通");
        assert!(result.confidence >= 80);
    }

    #[test]
    fn 文档站浏览应识别为资料阅读() {
        let result = classify_activity(
            "Google Chrome",
            "Tauri Guide",
            Some("https://tauri.app/zh-cn/develop/calling-rust/"),
        );

        assert_eq!(result.base_category, "browser");
        assert_eq!(result.semantic_category, "资料阅读");
        assert!(result.confidence >= 70);
    }

    #[test]
    fn 搜索引擎检索应识别为资料调研() {
        let result = classify_activity(
            "Google Chrome",
            "tauri onmoved event - Google 搜索",
            Some("https://www.google.com/search?q=tauri+onmoved+event"),
        );

        assert_eq!(result.semantic_category, "资料调研");
        assert!(result.confidence >= 70);
    }

    #[test]
    fn github_拉取请求页面应识别为编码开发() {
        let result = classify_activity(
            "Google Chrome",
            "Fix updater retry · Pull Request #12 · wm94i/Work_Review",
            Some("https://github.com/wm94i/Work_Review/pull/12"),
        );

        assert_eq!(result.semantic_category, "编码开发");
        assert!(result.confidence >= 60);
    }

    #[test]
    fn 浏览器任务看板应识别为任务规划() {
        let result = classify_activity(
            "Google Chrome",
            "Sprint 15 Board - Linear",
            Some("https://linear.app/work-review/board"),
        );

        assert_eq!(result.semantic_category, "任务规划");
        assert!(result.confidence >= 60);
    }

    #[test]
    fn 低证据活动应回退为未知活动() {
        let result = classify_activity("UnknownApp", "首页", None);

        assert_eq!(result.semantic_category, "未知活动");
        assert!(result.confidence < 60);
    }
}
