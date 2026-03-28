// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// macOS: objc 宏（msg_send!, class! 等）需要 macro_use 全局导入
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod activity_classifier;
mod analysis;
mod avatar_engine;
mod commands;
mod config;
mod database;
mod error;
mod idle_detector;
mod monitor;
mod ocr;
mod ocr_logger;
mod privacy;
mod screen_lock;
mod screenshot;
mod storage;
mod work_intelligence;

use chrono;
use config::AppConfig;
use database::Database;
use once_cell::sync::OnceCell;
use privacy::PrivacyFilter;
use screenshot::ScreenshotService;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use storage::StorageManager;
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};

// 全局 AppHandle，用于在 macOS Dock 点击时恢复窗口
static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_MENU_SHOW_ID: &str = "show";
const TRAY_MENU_RECORDING_TOGGLE_ID: &str = "recording-toggle";
const TRAY_MENU_LIGHTWEIGHT_MODE_ID: &str = "lightweight-mode";
const TRAY_MENU_AVATAR_TOGGLE_ID: &str = "avatar-toggle";
const TRAY_MENU_QUIT_ID: &str = "quit";
pub(crate) const RECORDING_STATE_CHANGED_EVENT: &str = "recording-state-changed";
pub(crate) const CONFIG_CHANGED_EVENT: &str = "config-changed";

type AppMenuItem = MenuItem<tauri::Wry>;
type AppCheckMenuItem = CheckMenuItem<tauri::Wry>;

pub(crate) struct TrayMenuState {
    recording_toggle: AppMenuItem,
    lightweight_mode: AppCheckMenuItem,
    avatar_toggle: AppCheckMenuItem,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingStatePayload {
    pub is_recording: bool,
    pub is_paused: bool,
}

#[cfg(target_os = "windows")]
pub(crate) fn build_windows_window_icon() -> Option<tauri::image::Image<'static>> {
    match image::load_from_memory_with_format(
        include_bytes!("../icons/windows-icon.png"),
        image::ImageFormat::Png,
    ) {
        Ok(decoded) => {
            let decoded = if decoded.width() > 256 || decoded.height() > 256 {
                decoded.resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
            } else {
                decoded
            };

            let rgba = decoded.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(tauri::image::Image::new_owned(
                rgba.into_raw(),
                width,
                height,
            ))
        }
        Err(e) => {
            log::warn!("加载 Windows 专用窗口图标失败，回退默认图标: {e}");
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainWindowCloseBehavior {
    HideToTray,
    CloseWindow,
}

fn main_window_close_behavior(lightweight_mode: bool) -> MainWindowCloseBehavior {
    if lightweight_mode {
        MainWindowCloseBehavior::CloseWindow
    } else {
        MainWindowCloseBehavior::HideToTray
    }
}

fn effective_dock_visibility(
    hide_dock_icon: bool,
    lightweight_mode: bool,
    has_main_window: bool,
) -> bool {
    !hide_dock_icon && (!lightweight_mode || has_main_window)
}

pub(crate) fn sync_effective_dock_visibility(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() else {
        return;
    };

    let (hide_dock_icon, lightweight_mode) = {
        let state = state.lock().unwrap_or_else(|e| e.into_inner());
        (state.config.hide_dock_icon, state.config.lightweight_mode)
    };
    let has_main_window = app.get_webview_window(MAIN_WINDOW_LABEL).is_some();
    let visible = effective_dock_visibility(hide_dock_icon, lightweight_mode, has_main_window);
    commands::apply_dock_visibility(visible, false);
}

pub(crate) fn configure_main_window(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    if let Some(icon) = build_windows_window_icon() {
        if let Err(e) = window.set_icon(icon) {
            log::warn!("设置 Windows 主窗口图标失败，继续使用默认图标: {e}");
        }
    }

    #[cfg(target_os = "macos")]
    {
        use tauri::TitleBarStyle;

        let _ = window.set_decorations(true);
        let _ = window.set_title_bar_style(TitleBarStyle::Transparent);
        configure_main_window_collection_behavior(window);
    }
}

#[cfg(target_os = "macos")]
fn configure_main_window_collection_behavior(window: &tauri::WebviewWindow) {
    use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
    use cocoa::base::id;

    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_window = ns_window as id;
            let mut behavior = ns_window.collectionBehavior();
            behavior |= NSWindowCollectionBehavior::NSWindowCollectionBehaviorMoveToActiveSpace;
            ns_window.setCollectionBehavior_(behavior);
        }
    }
}

fn align_window_to_reference_monitor(
    window: &tauri::WebviewWindow,
    reference_window: Option<&tauri::WebviewWindow>,
) {
    let Some(reference_window) = reference_window else {
        return;
    };

    let Ok(Some(reference_monitor)) = reference_window.current_monitor() else {
        return;
    };
    let Ok(window_size) = window.outer_size() else {
        return;
    };

    let work_area = reference_monitor.work_area();
    let monitor_width = work_area.size.width as i32;
    let monitor_height = work_area.size.height as i32;
    let window_width = window_size.width as i32;
    let window_height = window_size.height as i32;

    let target_x = work_area.position.x + ((monitor_width - window_width).max(0) / 2);
    let target_y = work_area.position.y + ((monitor_height - window_height).max(0) / 2);

    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        target_x, target_y,
    )));
}

pub(crate) fn ensure_main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, error::AppError> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == MAIN_WINDOW_LABEL)
        .or_else(|| app.config().app.windows.first())
        .ok_or_else(|| error::AppError::Unknown("未找到主窗口配置".to_string()))?;

    let window = tauri::WebviewWindowBuilder::from_config(app, window_config)
        .map_err(|e| error::AppError::Unknown(format!("创建主窗口构建器失败: {e}")))?
        .build()
        .map_err(|e| error::AppError::Unknown(format!("重建主窗口失败: {e}")))?;

    configure_main_window(&window);
    Ok(window)
}

pub(crate) fn reveal_main_window(
    app: &AppHandle,
    source_window_label: Option<&str>,
) -> Result<(), error::AppError> {
    let window = ensure_main_window(app)?;
    let reference_window = source_window_label.and_then(|label| app.get_webview_window(label));
    align_window_to_reference_monitor(&window, reference_window.as_ref());
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    sync_effective_dock_visibility(app);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordingToggleAction {
    Start,
    Pause,
    Resume,
}

fn tray_recording_toggle_action(is_recording: bool, is_paused: bool) -> RecordingToggleAction {
    if !is_recording {
        RecordingToggleAction::Start
    } else if is_paused {
        RecordingToggleAction::Resume
    } else {
        RecordingToggleAction::Pause
    }
}

fn tray_recording_toggle_label(is_recording: bool, is_paused: bool) -> &'static str {
    match tray_recording_toggle_action(is_recording, is_paused) {
        RecordingToggleAction::Start => "开始录制",
        RecordingToggleAction::Pause => "暂停录制",
        RecordingToggleAction::Resume => "恢复录制",
    }
}

pub(crate) fn refresh_tray_menu(app: &AppHandle) {
    let Some(tray_menu) = app.try_state::<TrayMenuState>() else {
        return;
    };
    let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() else {
        return;
    };

    let (is_recording, is_paused, lightweight_mode, avatar_enabled) = {
        let state = state.lock().unwrap_or_else(|e| e.into_inner());
        (
            state.is_recording,
            state.is_paused,
            state.config.lightweight_mode,
            state.config.avatar_enabled,
        )
    };

    let _ = tray_menu
        .recording_toggle
        .set_text(tray_recording_toggle_label(is_recording, is_paused));
    let _ = tray_menu.lightweight_mode.set_checked(lightweight_mode);
    let _ = tray_menu.avatar_toggle.set_checked(avatar_enabled);
}

pub(crate) fn emit_recording_state_changed(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() else {
        return;
    };

    let payload = {
        let state = state.lock().unwrap_or_else(|e| e.into_inner());
        RecordingStatePayload {
            is_recording: state.is_recording,
            is_paused: state.is_paused,
        }
    };

    let _ = app.emit(RECORDING_STATE_CHANGED_EVENT, payload);
    refresh_tray_menu(app);
}

pub(crate) fn emit_config_changed(app: &AppHandle, config: &AppConfig) {
    let _ = app.emit(CONFIG_CHANGED_EVENT, config);
    refresh_tray_menu(app);
}

fn build_tray_icon(app: &tauri::App) -> tauri::image::Image<'static> {
    #[cfg(target_os = "macos")]
    {
        match image::load_from_memory_with_format(
            include_bytes!("../icons/tray-template.png"),
            image::ImageFormat::Png,
        ) {
            Ok(decoded) => {
                let rgba = decoded.to_rgba8();
                let (width, height) = rgba.dimensions();
                tauri::image::Image::new_owned(rgba.into_raw(), width, height)
            }
            Err(e) => {
                log::warn!("加载 macOS 状态栏专用图标失败，回退默认图标: {e}");
                app.default_window_icon()
                    .expect("应用默认图标缺失")
                    .clone()
                    .to_owned()
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        app.default_window_icon()
            .expect("应用默认图标缺失")
            .clone()
            .to_owned()
    }
}

/// 应用状态
pub struct AppState {
    pub config: AppConfig,
    pub database: Database,
    pub privacy_filter: PrivacyFilter,
    pub screenshot_service: ScreenshotService,
    pub storage_manager: StorageManager,
    pub data_dir: PathBuf,
    pub config_path: PathBuf,
    pub is_recording: bool,
    pub is_paused: bool,
    pub avatar_state: avatar_engine::AvatarStatePayload,
    pub avatar_generating_report: bool,
}

#[derive(Default)]
pub(crate) struct AppLifecycleState {
    suppress_next_exit: bool,
    explicit_quit_requested: bool,
}

#[derive(Serialize, Deserialize)]
struct DataDirPreference {
    data_dir: String,
}

fn should_prevent_exit(suppress_next_exit: bool, explicit_quit_requested: bool) -> bool {
    suppress_next_exit && !explicit_quit_requested
}

pub(crate) fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join("work-review"))
        .unwrap_or_else(|| PathBuf::from("./data"))
}

fn data_dir_preference_path() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("work-review").join("data-location.json"))
        .unwrap_or_else(|| PathBuf::from("./work-review-data-location.json"))
}

fn load_data_dir_preference() -> Option<PathBuf> {
    let path = data_dir_preference_path();
    let content = std::fs::read_to_string(path).ok()?;
    let preference: DataDirPreference = serde_json::from_str(&content).ok()?;
    let data_dir = preference.data_dir.trim();
    if data_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(data_dir))
    }
}

pub(crate) fn save_data_dir_preference(data_dir: &Path) -> std::io::Result<()> {
    let default_dir = default_data_dir();
    let preference_path = data_dir_preference_path();

    if data_dir == default_dir {
        if preference_path.exists() {
            std::fs::remove_file(preference_path)?;
        }
        return Ok(());
    }

    if let Some(parent) = preference_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(&DataDirPreference {
        data_dir: data_dir.to_string_lossy().to_string(),
    })
    .map_err(std::io::Error::other)?;

    std::fs::write(preference_path, content)?;
    Ok(())
}

fn ensure_data_dir(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(path)?;
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

/// 获取数据目录
fn resolve_data_dir() -> PathBuf {
    let default_dir = default_data_dir();
    let preferred_dir = load_data_dir_preference().unwrap_or_else(|| default_dir.clone());

    match ensure_data_dir(&preferred_dir) {
        Ok(dir) => {
            migrate_legacy_data_dir(&dir);
            dir
        }
        Err(error) => {
            log::warn!("创建数据目录失败，回退默认目录: {error}");

            if preferred_dir != default_dir {
                if let Ok(dir) = ensure_data_dir(&default_dir) {
                    migrate_legacy_data_dir(&dir);
                    let _ = save_data_dir_preference(&dir);
                    return dir;
                }
            }

            let fallback_dir = PathBuf::from("./data");
            if let Err(fallback_error) = std::fs::create_dir_all(&fallback_dir) {
                log::warn!("创建兜底数据目录失败: {fallback_error}");
            }
            migrate_legacy_data_dir(&fallback_dir);
            fallback_dir
        }
    }
}

fn migrate_legacy_data_dir(target_dir: &PathBuf) {
    let legacy_dir = match std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("data")))
    {
        Some(path) => path,
        None => return,
    };

    if legacy_dir == *target_dir || !legacy_dir.exists() {
        return;
    }

    let target_has_data = target_dir.join("config.json").exists()
        || target_dir.join("workreview.db").exists()
        || target_dir.join("screenshots").exists();
    if target_has_data {
        return;
    }

    if let Err(error) = copy_dir_contents(&legacy_dir, target_dir, false) {
        log::warn!("迁移旧版数据目录失败: {error}");
    } else {
        log::info!("已将旧版数据目录迁移到稳定目录: {:?}", target_dir);
    }
}

pub(crate) fn copy_dir_contents(
    from: &Path,
    to: &Path,
    overwrite_existing: bool,
) -> Result<u64, std::io::Error> {
    std::fs::create_dir_all(to)?;
    let mut copied_files = 0;

    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = to.join(entry.file_name());

        if source_path.is_dir() {
            copied_files += copy_dir_contents(&source_path, &target_path, overwrite_existing)?;
            continue;
        }

        if overwrite_existing || !target_path.exists() {
            std::fs::copy(&source_path, &target_path)?;
            copied_files += 1;
        }
    }

    Ok(copied_files)
}

/// 浏览器 URL 采集偶发失败时，尝试从最近同窗口标题的活动里恢复 URL。
/// 这是近似统计兜底：优先减少同一页面被切碎成多段或掉成 0 站点 0 页面。
fn recover_recent_browser_url(
    database: &Database,
    app_name: &str,
    window_title: &str,
    now_ts: i64,
    max_age_secs: i64,
) -> Option<String> {
    if !monitor::is_browser_app(app_name) || window_title.is_empty() {
        return None;
    }

    database
        .get_latest_activity_by_app_title(app_name, window_title)
        .ok()
        .flatten()
        .and_then(|activity| {
            let age = now_ts - activity.timestamp;
            if age <= max_age_secs {
                activity.browser_url.filter(|url| !url.is_empty())
            } else {
                None
            }
        })
}

pub(crate) fn resolve_activity_classification(
    config: &AppConfig,
    app_name: &str,
    window_title: &str,
    browser_url: Option<&str>,
) -> activity_classifier::ActivityClassification {
    let base_category =
        monitor::categorize_app_with_rules(&config.app_category_rules, app_name, window_title);
    activity_classifier::classify_activity_with_base_category(
        app_name,
        window_title,
        browser_url,
        &base_category,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecordingLoopDecision {
    should_continue: bool,
    screenshot_interval: u64,
    reset_capture_clock: bool,
}

fn recording_loop_decision(
    is_recording: bool,
    is_paused: bool,
    screenshot_interval: u64,
) -> RecordingLoopDecision {
    if !is_recording || is_paused {
        RecordingLoopDecision {
            should_continue: false,
            screenshot_interval: 1,
            reset_capture_clock: true,
        }
    } else {
        RecordingLoopDecision {
            should_continue: true,
            screenshot_interval,
            reset_capture_clock: false,
        }
    }
}

fn monitoring_poll_interval_ms() -> u64 {
    500
}

fn avatar_monitor_poll_interval_ms() -> u64 {
    180
}

#[derive(Debug, Clone, PartialEq)]
struct AvatarActivityDecision {
    should_continue: bool,
    reset_state: Option<avatar_engine::AvatarStatePayload>,
}

fn avatar_activity_decision(
    avatar_enabled: bool,
    is_recording: bool,
    is_paused: bool,
    avatar_opacity: f64,
) -> AvatarActivityDecision {
    if !avatar_enabled {
        return AvatarActivityDecision {
            should_continue: false,
            reset_state: Some(avatar_engine::default_avatar_state()),
        };
    }

    if !is_recording || is_paused {
        return AvatarActivityDecision {
            should_continue: false,
            reset_state: Some(avatar_engine::apply_avatar_opacity(
                avatar_engine::default_avatar_state(),
                avatar_opacity,
            )),
        };
    }

    AvatarActivityDecision {
        should_continue: true,
        reset_state: None,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AvatarTransitionDecision {
    emit_state: Option<avatar_engine::AvatarStatePayload>,
    pending_state: Option<avatar_engine::AvatarStatePayload>,
    pending_hits: u8,
}

fn avatar_transition_decision(
    current: Option<&avatar_engine::AvatarStatePayload>,
    pending: Option<&avatar_engine::AvatarStatePayload>,
    pending_hits: u8,
    candidate: &avatar_engine::AvatarStatePayload,
) -> AvatarTransitionDecision {
    const AVATAR_MODE_STABILITY_THRESHOLD: u8 = 2;

    match current {
        None => AvatarTransitionDecision {
            emit_state: Some(candidate.clone()),
            pending_state: None,
            pending_hits: 0,
        },
        Some(current_state) if current_state == candidate => AvatarTransitionDecision {
            emit_state: None,
            pending_state: None,
            pending_hits: 0,
        },
        Some(current_state) if current_state.mode == candidate.mode => AvatarTransitionDecision {
            emit_state: Some(candidate.clone()),
            pending_state: None,
            pending_hits: 0,
        },
        Some(_) => {
            let next_hits = if pending == Some(candidate) {
                pending_hits.saturating_add(1)
            } else {
                1
            };

            if next_hits >= AVATAR_MODE_STABILITY_THRESHOLD {
                AvatarTransitionDecision {
                    emit_state: Some(candidate.clone()),
                    pending_state: None,
                    pending_hits: 0,
                }
            } else {
                AvatarTransitionDecision {
                    emit_state: None,
                    pending_state: Some(candidate.clone()),
                    pending_hits: next_hits,
                }
            }
        }
    }
}

fn should_skip_transient_window(active_window: &monitor::ActiveWindow) -> bool {
    let app_lower = active_window.app_name.to_lowercase();
    matches!(
        app_lower.as_str(),
        "dock"
            | "systemuiserver"
            | "control center"
            | "spotlight"
            | "notificationcenter"
            | "loginwindow"
            | "screencaptureui"
            | "universalaccessauthwarn"
            | "windowmanager"
            | "wallpaper"
    )
}

fn should_skip_system_window(active_window: &monitor::ActiveWindow) -> bool {
    let is_sys = monitor::is_system_process(&active_window.app_name);
    let is_explorer_shell = {
        let name_lower = active_window.app_name.to_lowercase();
        let name_trimmed = name_lower.trim_end_matches(".exe");
        (name_trimmed == "explorer" || name_trimmed == "file explorer")
            && active_window.window_title.is_empty()
    };

    is_sys || is_explorer_shell
}

async fn background_avatar_task(state: Arc<Mutex<AppState>>, app: AppHandle) {
    let mut last_avatar_state: Option<avatar_engine::AvatarStatePayload> = None;
    let mut pending_avatar_state: Option<avatar_engine::AvatarStatePayload> = None;
    let mut pending_avatar_hits: u8 = 0;
    let mut last_window_signature: Option<String> = None;
    const IDLE_TIMEOUT_MINUTES: u64 = 3;
    let idle_detector = idle_detector::IdleDetector::new(IDLE_TIMEOUT_MINUTES);

    loop {
        tokio::time::sleep(Duration::from_millis(avatar_monitor_poll_interval_ms())).await;

        let (avatar_enabled, avatar_generating_report, avatar_opacity, is_recording, is_paused) = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            (
                state_guard.config.avatar_enabled,
                state_guard.avatar_generating_report,
                state_guard.config.avatar_opacity,
                state_guard.is_recording,
                state_guard.is_paused,
            )
        };

        let activity_decision =
            avatar_activity_decision(avatar_enabled, is_recording, is_paused, avatar_opacity);
        if !activity_decision.should_continue {
            pending_avatar_state = None;
            pending_avatar_hits = 0;
            last_window_signature = None;

            if let Some(reset_state) = activity_decision.reset_state {
                let should_emit_reset = last_avatar_state.as_ref() != Some(&reset_state);
                {
                    let mut state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    state_guard.avatar_state = reset_state.clone();
                }

                if avatar_enabled && should_emit_reset {
                    avatar_engine::emit_avatar_state(&app, &reset_state);
                }

                last_avatar_state = Some(reset_state);
            } else {
                last_avatar_state = None;
            }
            continue;
        }

        let sampled_at = std::time::Instant::now();
        let active_window = match monitor::get_active_window_fast() {
            Ok(window) => window,
            Err(_) => continue,
        };

        let app_category_rules = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            state_guard.config.app_category_rules.clone()
        };

        if should_skip_transient_window(&active_window) || should_skip_system_window(&active_window)
        {
            continue;
        }

        let input_idle = idle_detector.is_input_idle();
        let avatar_state = avatar_engine::apply_avatar_opacity(
            avatar_engine::derive_avatar_state_with_rules(
                &app_category_rules,
                &active_window.app_name,
                &active_window.window_title,
                active_window.browser_url.as_deref(),
                input_idle,
                avatar_generating_report,
            ),
            avatar_opacity,
        );

        let window_signature = format!("{}|{}", active_window.app_name, active_window.window_title);
        let transition_decision = avatar_transition_decision(
            last_avatar_state.as_ref(),
            pending_avatar_state.as_ref(),
            pending_avatar_hits,
            &avatar_state,
        );

        pending_avatar_state = transition_decision.pending_state;
        pending_avatar_hits = transition_decision.pending_hits;

        if let Some(next_avatar_state) = transition_decision.emit_state {
            let collect_cost_ms = sampled_at.elapsed().as_millis();
            let previous_mode = last_avatar_state
                .as_ref()
                .map(|state| state.mode.as_str())
                .unwrap_or("none");

            {
                let mut state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard.avatar_state = next_avatar_state.clone();
            }

            avatar_engine::emit_avatar_state(&app, &next_avatar_state);

            let entered_idle = match &last_avatar_state {
                Some(previous) => !previous.is_idle && next_avatar_state.is_idle,
                None => next_avatar_state.is_idle,
            };

            if entered_idle {
                avatar_engine::emit_avatar_bubble(
                    &app,
                    &avatar_engine::AvatarBubblePayload::info("先放松一下，待会再继续推进。"),
                );
            }

            log::info!(
                "🐾 桌宠状态切换: {} -> {} | 窗口={} | 采集耗时={}ms",
                previous_mode,
                next_avatar_state.mode,
                window_signature,
                collect_cost_ms
            );

            last_avatar_state = Some(next_avatar_state);
            last_window_signature = Some(window_signature);
        } else if last_window_signature.as_deref() != Some(window_signature.as_str()) {
            log::debug!(
                "🐾 桌宠检测到前台切换，但状态未变: {} | 采集耗时={}ms",
                window_signature,
                sampled_at.elapsed().as_millis()
            );
            last_window_signature = Some(window_signature);
        }
    }
}

// 系统托盘在 setup 钩子中使用 TrayIconBuilder 创建 (Tauri v2)

/// 后台截屏任务
/// 使用 Arc<Mutex<AppState>> 而非 tauri::State，因为 State 无法在 async move 块中手动构造
async fn background_screenshot_task(state: Arc<Mutex<AppState>>, app: AppHandle) {
    // ===== 状态变量 =====
    let mut last_app_name: Option<String> = None;
    let mut last_app_window_title: Option<String> = None;
    let mut last_browser_url: Option<String> = None;

    let mut last_capture_time = std::time::Instant::now();

    // ===== 空闲检测器 =====
    // 固定 3 分钟空闲阈值：无键鼠操作且屏幕内容无变化时暂停计时
    const IDLE_TIMEOUT_MINUTES: u64 = 3;
    let idle_detector = idle_detector::IdleDetector::new(IDLE_TIMEOUT_MINUTES);
    let mut last_idle_log_time = std::time::Instant::now();
    let mut is_currently_idle = false; // 当前是否处于空闲状态

    const MIN_CAPTURE_INTERVAL_MS: u128 = 3000; // 最小截图间隔3秒（防抖）
    let poll_interval_ms = monitoring_poll_interval_ms(); // 桌宠状态和窗口切换检测优先更快反馈

    // OCR 并发限制：最多 2 个 OCR 任务同时运行，防止任务堆积消耗内存
    let ocr_semaphore = Arc::new(tokio::sync::Semaphore::new(2));

    // 锁屏检测器（无内部状态，复用同一实例避免重复分配）
    let screen_lock_monitor = screen_lock::ScreenLockMonitor::new();

    loop {
        // 检测屏幕锁定状态，锁屏时不统计时长
        if screen_lock_monitor.is_locked() {
            log::info!("🔒 屏幕已锁定，暂停活动统计");
            last_app_name = None; // 重置应用状态，解锁后视为新开始
            last_capture_time = std::time::Instant::now(); // 重置截图计时，避免解锁后累加锁屏时长
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // 首先检查录制状态并获取配置
        let decision = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            recording_loop_decision(
                state_guard.is_recording,
                state_guard.is_paused,
                state_guard.config.screenshot_interval,
            )
        };

        if decision.reset_capture_clock {
            last_capture_time = std::time::Instant::now();
        }

        if !decision.should_continue {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let screenshot_interval = decision.screenshot_interval;

        // 轮询检测活动窗口（1秒间隔），让桌宠状态切换更及时
        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;

        // 获取当前活动窗口
        // 失败原因：Windows 睡眠/待机/UAC 时无前台窗口、macOS 权限不足等
        // 此时重置计时器，避免累积的时长被错误归属到下一个真实应用
        let mut active_window = match monitor::get_active_window() {
            Ok(w) => w,
            Err(_) => {
                last_capture_time = std::time::Instant::now();
                continue;
            }
        };

        // 再次检查状态
        let should_capture = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            state_guard.is_recording && !state_guard.is_paused
        };

        if !should_capture {
            continue;
        }

        // macOS 系统进程在用户切换应用、点击 Dock 时会短暂成为前台应用
        // 跳过这些进程避免它们偷走其他应用的使用时长
        // 不更新 last_app_name，时长会在下一个正常轮询中通过 elapsed_secs 自然回收
        {
            if should_skip_transient_window(&active_window) {
                log::debug!("跳过系统瞬态进程: {}", active_window.app_name);
                continue;
            }
        }

        // 跳过系统 shell / 锁屏 / 桌面进程，避免睡眠/唤醒时累积虚假时长
        // 注意 explorer 特殊处理：有窗口标题时是文件管理器，应该记录
        {
            if should_skip_system_window(&active_window) {
                log::debug!(
                    "跳过系统进程: {} (title={})",
                    active_window.app_name,
                    active_window.window_title
                );
                continue;
            }
        }

        // 浏览器 URL 存在瞬时采集失败时，尽量复用同窗口最近一次成功值，减少统计断裂。
        const BROWSER_URL_STICKY_GAP_SECS: i64 = 120;
        if active_window.browser_url.is_none()
            && monitor::is_browser_app(&active_window.app_name)
            && !active_window.window_title.is_empty()
        {
            let now_ts = chrono::Local::now().timestamp();

            let recovered_url = if last_app_name.as_deref() == Some(active_window.app_name.as_str())
                && last_app_window_title.as_deref() == Some(active_window.window_title.as_str())
            {
                last_browser_url.clone()
            } else {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                recover_recent_browser_url(
                    &state_guard.database,
                    &active_window.app_name,
                    &active_window.window_title,
                    now_ts,
                    BROWSER_URL_STICKY_GAP_SECS,
                )
            };

            if let Some(recovered_url) = recovered_url {
                log::debug!(
                    "恢复浏览器 URL: {} | {} -> {}",
                    active_window.app_name,
                    active_window.window_title,
                    recovered_url
                );
                active_window.browser_url = Some(recovered_url);
            }
        }

        // ===== 检测应用切换 =====
        let previous_window_title = last_app_window_title.clone();
        let previous_browser_url = last_browser_url.clone();

        let url_changed = match (&last_browser_url, &active_window.browser_url) {
            (Some(l), Some(r)) => l != r,
            (None, None) => false,
            _ => true,
        };

        // 只有当两个标题不同时才算切换
        let title_changed = match (&last_app_window_title, &active_window.window_title) {
            (Some(last_title), active_title) => last_title != active_title,
            (None, _) => true,
        };

        let app_changed = match &last_app_name {
            Some(last) => last != &active_window.app_name || url_changed || title_changed,
            None => true,
        };
        // 保存切换前的应用名，用于时长归属修正
        let previous_app_name = if app_changed {
            last_app_name.clone()
        } else {
            None
        };

        // 计算距离上次截图的时间
        let elapsed_since_capture = last_capture_time.elapsed();
        let elapsed_secs = elapsed_since_capture.as_secs();

        // ===== 应用切换日志 =====
        if app_changed && last_app_name.is_some() {
            log::info!(
                "📊 应用切换: {} [{}] → {} [{}]",
                last_app_name.as_deref().unwrap_or("无"),
                previous_window_title.as_deref().unwrap_or(""),
                &active_window.app_name,
                &active_window.window_title,
            );
        }

        // ===== 空闲检测第一阶段：键鼠活动检查 =====
        let input_idle = idle_detector.is_input_idle();

        // 每 30 秒打印一次空闲状态日志（避免刷屏）
        if last_idle_log_time.elapsed() >= Duration::from_secs(30) {
            if input_idle != is_currently_idle {
                if input_idle {
                    log::info!("⏸️  键鼠超时，等待截图确认空闲状态...");
                } else {
                    log::info!("▶️  检测到用户活动，恢复正常记录");
                    idle_detector.reset();
                }
                is_currently_idle = input_idle;
            }
            last_idle_log_time = std::time::Instant::now();
        }

        // ===== 判断是否截图 =====
        // 1. 定时触发：到达配置的间隔时间
        // 2. 应用切换触发：满足最小间隔
        let should_take_screenshot = if elapsed_secs >= screenshot_interval {
            log::debug!("定时截图触发");
            true
        } else if app_changed && elapsed_since_capture.as_millis() >= MIN_CAPTURE_INTERVAL_MS {
            log::debug!("应用切换截图触发");
            true
        } else {
            false
        };

        // 保存 app_name 副本供浮动窗口检测使用（在 move 之前）
        let frontmost_app_name = active_window.app_name.clone();

        if !should_take_screenshot {
            // 如果是因为冷却时间未到而没有截图，但应用/标签页实际上已经变化了
            // 那么我们不要更新 last_* 变量，这样下一个轮询周期 app_changed 仍然为 true
            if !app_changed {
                last_app_name = Some(active_window.app_name.clone());
                last_app_window_title = Some(active_window.window_title.clone());
                last_browser_url = active_window.browser_url.clone();
            }
            continue;
        }

        // 取决定截图后，才更新上一个应用的信息
        last_app_name = Some(active_window.app_name.clone());
        last_app_window_title = Some(active_window.window_title.clone());
        last_browser_url = active_window.browser_url.clone();

        // 更新截图时间
        last_capture_time = std::time::Instant::now();

        // 使用距离上次截图的实际经过时间作为本次记录的时长
        // 而非固定的轮询间隔，避免截图间隔大于轮询间隔时丢失时长
        let (privacy_action, duration_to_record) = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            let action = state_guard.privacy_filter.check_privacy_full(
                &active_window.app_name,
                &active_window.window_title,
                active_window.browser_url.as_deref(),
            );
            // elapsed_secs 是距离上次截图的真实秒数，确保时长不丢失
            let duration = elapsed_secs.max(1) as i64;
            (action, duration)
        };
        // 锁已释放

        use privacy::PrivacyAction;
        let result: Option<database::Activity> = match privacy_action {
            PrivacyAction::Skip => {
                log::debug!(
                    "完全跳过: {} - {}",
                    active_window.app_name,
                    active_window.window_title
                );
                None
            }
            PrivacyAction::Anonymize => {
                log::debug!(
                    "内容脱敏: {} - {}",
                    active_window.app_name,
                    active_window.window_title
                );
                let classification = {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    crate::resolve_activity_classification(
                        &state_guard.config,
                        &active_window.app_name,
                        &active_window.window_title,
                        active_window.browser_url.as_deref(),
                    )
                };
                let activity = database::Activity {
                    id: None,
                    timestamp: chrono::Local::now().timestamp(),
                    app_name: active_window.app_name,
                    window_title: "[内容已脱敏]".to_string(),
                    screenshot_path: String::new(),
                    ocr_text: None,
                    category: classification.base_category,
                    duration: duration_to_record,
                    browser_url: None,
                    executable_path: active_window.executable_path,
                    semantic_category: Some(classification.semantic_category),
                    semantic_confidence: Some(i32::from(classification.confidence)),
                };

                // 短暂获取锁写入数据库
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                match state_guard.database.insert_activity(&activity) {
                    Ok(_) => Some(activity),
                    Err(e) => {
                        log::error!("保存活动记录失败: {e}");
                        None
                    }
                }
            }
            PrivacyAction::Record => {
                let classification = {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    crate::resolve_activity_classification(
                        &state_guard.config,
                        &active_window.app_name,
                        &active_window.window_title,
                        active_window.browser_url.as_deref(),
                    )
                };
                let category = classification.base_category.clone();
                let current_timestamp = chrono::Local::now().timestamp();

                // ===== 应用切换时长归属修正 =====
                // 切换应用时，上次截图到现在的时长应归属于上一个应用
                // 新应用从 0 开始计时，避免"偷"到上个应用的使用时长
                let adjusted_duration = if app_changed {
                    if let Some(ref prev_app) = previous_app_name {
                        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                        let prev_activity = if let Some(prev_url) = previous_browser_url
                            .as_deref()
                            .filter(|url| !url.is_empty())
                        {
                            state_guard
                                .database
                                .get_latest_activity_by_url(prev_url)
                                .ok()
                                .flatten()
                        } else if monitor::is_browser_app(prev_app) {
                            previous_window_title
                                .as_deref()
                                .filter(|title| !title.is_empty())
                                .and_then(|title| {
                                    state_guard
                                        .database
                                        .get_latest_activity_by_app_title(prev_app, title)
                                        .ok()
                                        .flatten()
                                })
                                .or_else(|| {
                                    state_guard
                                        .database
                                        .get_latest_activity_by_app(prev_app)
                                        .ok()
                                        .flatten()
                                })
                        } else {
                            state_guard
                                .database
                                .get_latest_activity_by_app(prev_app)
                                .ok()
                                .flatten()
                        };

                        if let Some(prev_activity) = prev_activity {
                            if let Some(prev_id) = prev_activity.id {
                                let _ = state_guard.database.merge_activity(
                                    prev_id,
                                    duration_to_record,
                                    None,
                                    &prev_activity.screenshot_path,
                                    current_timestamp,
                                );
                                log::debug!(
                                    "⏱️ 时长回补: {} +{}s (切换到 {})",
                                    prev_app,
                                    duration_to_record,
                                    active_window.app_name
                                );
                            }
                        }
                    }
                    0i64
                } else {
                    duration_to_record
                };

                // 先检查是否有可合并的记录（在截屏之前判断，避免不必要的截图保存）
                let latest_activity = {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(url) = active_window
                        .browser_url
                        .as_deref()
                        .filter(|url| !url.is_empty())
                    {
                        state_guard
                            .database
                            .get_latest_activity_by_url(url)
                            .ok()
                            .flatten()
                    } else if monitor::is_browser_app(&active_window.app_name)
                        && !active_window.window_title.is_empty()
                    {
                        state_guard
                            .database
                            .get_latest_activity_by_app_title(
                                &active_window.app_name,
                                &active_window.window_title,
                            )
                            .ok()
                            .flatten()
                    } else {
                        state_guard
                            .database
                            .get_latest_activity_by_app(&active_window.app_name)
                            .ok()
                            .flatten()
                    }
                };

                // "Unknown" 进程名不做合并：无法区分是哪个进程，强制新建
                // 防止所有识别失败的进程时长累积到同一条记录导致统计失真
                // 时间间隔超过 10 分钟也不合并：上午/下午用同一个 app 属于不同工作段
                const MERGE_GAP_SECS: i64 = 600;
                let is_merge = if let Some(ref latest) = latest_activity {
                    let mut merge = active_window.app_name != "Unknown"
                        && (current_timestamp - latest.timestamp) <= MERGE_GAP_SECS;

                    // 如果由于某种原因 browser_url 获取失败，但它确实是一个浏览器
                    // 我们必须强制让 window_title 完全相同才能合并，否则不同标签页的切换会被死死合并成一条记录。
                    if merge
                        && active_window.browser_url.is_none()
                        && monitor::is_browser_app(&active_window.app_name)
                        && latest.window_title != active_window.window_title
                    {
                        merge = false;
                    }

                    merge
                } else {
                    false
                };

                if is_merge {
                    // === 合并路径：不保存截图，只做 OCR ===
                    let latest = latest_activity.unwrap();
                    let latest_id = latest.id.unwrap();

                    // 截屏到内存，保存为临时文件供 OCR 使用
                    let screenshot_result = {
                        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                        state_guard
                            .screenshot_service
                            .capture_for_window(Some(&active_window))
                    };

                    // ===== 空闲检测第二阶段：截图哈希确认 =====
                    // 只有键鼠超时时才检查屏幕变化，避免正常使用时的额外计算
                    let is_confirmed_idle = if input_idle {
                        if let Ok(ref screenshot) = screenshot_result {
                            let hash = screenshot::ScreenshotService::calculate_image_hash(
                                &screenshot.path,
                            )
                            .unwrap_or(0);
                            idle_detector.confirm_idle_with_hash(hash)
                        } else {
                            false
                        }
                    } else {
                        // 有键鼠活动，重置空闲检测器
                        idle_detector.reset();
                        false
                    };

                    // 如果确认空闲，跳过时长记录
                    let effective_duration = if is_confirmed_idle {
                        log::debug!("空闲确认: 跳过本次时长记录");
                        0
                    } else {
                        adjusted_duration
                    };

                    // 合并记录（不更新 screenshot_path，保留活动创建时的原始截图）
                    // 即使 effective_duration 为 0，也需要更新时间戳以保持记录活跃
                    if effective_duration > 0 {
                        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                        match state_guard.database.merge_activity(
                            latest_id,
                            effective_duration,
                            None,
                            &latest.screenshot_path, // 保留原始截图路径不变
                            current_timestamp,
                        ) {
                            Ok(_) => {
                                log::info!(
                                    "✅ 合并成功: {} (id={}, 新时长={}s)",
                                    active_window.app_name,
                                    latest_id,
                                    latest.duration + effective_duration
                                );
                            }
                            Err(e) => {
                                log::error!("合并活动记录失败: {e}");
                            }
                        }
                    }

                    // 对截图执行 OCR，然后删除临时截图
                    if let Ok(screenshot) = screenshot_result {
                        let temp_path = screenshot.path.clone();
                        let state_clone = state.clone();
                        let data_dir_clone = {
                            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                            state_guard.data_dir.clone()
                        };

                        use std::sync::atomic::{AtomicU64, Ordering};
                        static MERGE_SCREENSHOT_HASH: AtomicU64 = AtomicU64::new(0);

                        let ocr_sem = ocr_semaphore.clone();

                        tokio::spawn(async move {
                            // 非阻塞获取 permit，满载时跳过 OCR 避免任务堆积
                            let _permit = match ocr_sem.try_acquire_owned() {
                                Ok(p) => p,
                                Err(_) => {
                                    log::debug!("OCR 并发已满，跳过合并路径 OCR");
                                    let _ = std::fs::remove_file(&temp_path);
                                    return;
                                }
                            };

                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                            // 计算哈希做去重判断
                            let current_hash =
                                screenshot::ScreenshotService::calculate_image_hash(&temp_path)
                                    .unwrap_or(0);
                            let last_hash =
                                MERGE_SCREENSHOT_HASH.swap(current_hash, Ordering::Relaxed);

                            let should_ocr = if last_hash != 0 {
                                let similarity = screenshot::ScreenshotService::hash_similarity(
                                    last_hash,
                                    current_hash,
                                );
                                if similarity > 90 {
                                    log::debug!("合并截图相似度 {similarity}%，跳过 OCR");
                                    false
                                } else {
                                    log::debug!("合并截图相似度 {similarity}%，执行 OCR");
                                    true
                                }
                            } else {
                                true
                            };

                            if should_ocr {
                                let ocr_service = ocr::OcrService::new(&data_dir_clone);
                                if let Ok(Some(ocr_result)) = ocr_service.extract_text(&temp_path) {
                                    if !ocr_result.text.is_empty() {
                                        let filtered_text =
                                            ocr::filter_sensitive_text(&ocr_result.text);
                                        if let Ok(state_guard) = state_clone.lock() {
                                            let _ = state_guard.database.update_activity_ocr(
                                                latest_id,
                                                Some(filtered_text),
                                            );
                                            log::info!(
                                                "OCR 完成(合并): 活动 {} 识别到 {} 个字符",
                                                latest_id,
                                                ocr_result.text.len()
                                            );
                                        }
                                    }
                                }
                            }

                            // 删除临时截图文件（不保留）
                            let _ = std::fs::remove_file(&temp_path);
                            log::debug!("已删除临时截图: {temp_path:?}");
                        });
                    }

                    Some(database::Activity {
                        id: Some(latest_id),
                        timestamp: current_timestamp,
                        app_name: active_window.app_name.clone(),
                        window_title: active_window.window_title,
                        screenshot_path: latest.screenshot_path,
                        ocr_text: None,
                        category,
                        duration: latest.duration + effective_duration,
                        browser_url: active_window.browser_url,
                        executable_path: active_window.executable_path,
                        semantic_category: Some(classification.semantic_category.clone()),
                        semantic_confidence: Some(i32::from(classification.confidence)),
                    })
                } else {
                    // === 新建路径：正常截屏并保存 ===
                    let screenshot_result = {
                        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                        state_guard
                            .screenshot_service
                            .capture_for_window(Some(&active_window))
                    };

                    match screenshot_result {
                        Ok(screenshot_result) => {
                            // ===== 空闲检测第二阶段：截图哈希确认 =====
                            let is_confirmed_idle = if input_idle {
                                let hash = screenshot::ScreenshotService::calculate_image_hash(
                                    &screenshot_result.path,
                                )
                                .unwrap_or(0);
                                idle_detector.confirm_idle_with_hash(hash)
                            } else {
                                idle_detector.reset();
                                false
                            };

                            // 如果确认空闲，跳过时长记录（但仍创建活动记录以保持截图）
                            let effective_duration = if is_confirmed_idle {
                                log::debug!("空闲确认: 新活动时长设为 0");
                                0
                            } else {
                                adjusted_duration
                            };

                            let (relative_path, data_dir_clone) = {
                                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                                (
                                    state_guard
                                        .screenshot_service
                                        .get_relative_path(&screenshot_result.path),
                                    state_guard.data_dir.clone(),
                                )
                            };

                            let activity = database::Activity {
                                id: None,
                                timestamp: screenshot_result.timestamp,
                                app_name: active_window.app_name.clone(),
                                window_title: active_window.window_title,
                                screenshot_path: relative_path.clone(),
                                ocr_text: None,
                                category,
                                duration: effective_duration,
                                browser_url: active_window.browser_url,
                                executable_path: active_window.executable_path,
                                semantic_category: Some(classification.semantic_category.clone()),
                                semantic_confidence: Some(i32::from(classification.confidence)),
                            };

                            let inserted = {
                                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                                state_guard.database.insert_activity(&activity)
                            };

                            match inserted {
                                Ok(activity_id) => {
                                    log::info!(
                                        "📝 新建活动: {} (id={})",
                                        active_window.app_name,
                                        activity_id
                                    );

                                    // 异步 OCR（新建活动的截图已保存，不删除）
                                    let state_clone = state.clone();
                                    let screenshot_path_clone = relative_path;
                                    let ocr_sem = ocr_semaphore.clone();
                                    tokio::spawn(async move {
                                        // 非阻塞获取 permit，满载时跳过 OCR
                                        let _permit = match ocr_sem.try_acquire_owned() {
                                            Ok(p) => p,
                                            Err(_) => {
                                                log::debug!("OCR 并发已满，跳过新建路径 OCR");
                                                return;
                                            }
                                        };

                                        tokio::time::sleep(tokio::time::Duration::from_secs(1))
                                            .await;

                                        let full_path = data_dir_clone.join(&screenshot_path_clone);
                                        let ocr_service = ocr::OcrService::new(&data_dir_clone);

                                        if let Ok(Some(ocr_result)) =
                                            ocr_service.extract_text(&full_path)
                                        {
                                            if !ocr_result.text.is_empty() {
                                                let filtered_text =
                                                    ocr::filter_sensitive_text(&ocr_result.text);
                                                if let Ok(state_guard) = state_clone.lock() {
                                                    let _ =
                                                        state_guard.database.update_activity_ocr(
                                                            activity_id,
                                                            Some(filtered_text),
                                                        );
                                                    log::info!(
                                                        "OCR 完成(新建): 活动 {} 识别到 {} 个字符",
                                                        activity_id,
                                                        ocr_result.text.len()
                                                    );
                                                }
                                            }
                                        }
                                    });

                                    Some(database::Activity {
                                        id: Some(activity_id),
                                        ..activity
                                    })
                                }
                                Err(e) => {
                                    log::error!("保存活动记录失败: {e}");
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("截屏失败: {e}");
                            None
                        }
                    }
                }
            }
        };

        // 发送事件到前端
        if let Some(activity) = result {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                let _ = window.emit("screenshot-taken", &activity);
            }
        }

        // ===== 浮动窗口（PiP 画中画）检测 =====
        // 检测 layer > 0 的浮动窗口（如视频小窗），为它们记录使用时长
        // 浮动窗口不截图（截图已由主活动管理），仅记录时长
        let overlay_windows = monitor::get_overlay_windows(&frontmost_app_name);
        for ow in &overlay_windows {
            // 隐私检查
            let ow_privacy = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard
                    .privacy_filter
                    .check_privacy(&ow.app_name, &ow.window_title)
            };

            if ow_privacy == privacy::PrivacyAction::Skip {
                log::debug!("浮动窗口跳过(隐私): {}", ow.app_name);
                continue;
            }

            let classification = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                crate::resolve_activity_classification(
                    &state_guard.config,
                    &ow.app_name,
                    &ow.window_title,
                    ow.browser_url.as_deref(),
                )
            };
            let ow_category = classification.base_category.clone();
            let current_ts = chrono::Local::now().timestamp();
            let ow_duration = poll_interval_ms.div_ceil(1000) as i64;

            // 查找该应用的最近活动记录，尝试合并
            let latest = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard
                    .database
                    .get_latest_activity_by_app(&ow.app_name)
                    .ok()
                    .flatten()
            };

            const OW_MERGE_GAP_SECS: i64 = 600;
            let can_merge = if let Some(ref act) = latest {
                ow.app_name != "Unknown" && (current_ts - act.timestamp) <= OW_MERGE_GAP_SECS
            } else {
                false
            };

            if can_merge {
                let act = latest.unwrap();
                if let Some(act_id) = act.id {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    match state_guard.database.merge_activity(
                        act_id,
                        ow_duration,
                        None,
                        &act.screenshot_path,
                        current_ts,
                    ) {
                        Ok(_) => {
                            log::info!(
                                "🪟 浮动窗口合并: {} (id={}, +{}s, 总{}s)",
                                ow.app_name,
                                act_id,
                                ow_duration,
                                act.duration + ow_duration
                            );
                        }
                        Err(e) => log::error!("浮动窗口合并失败: {e}"),
                    }
                }
            } else {
                // 新建活动记录（无截图）
                let ow_title = if ow_privacy == privacy::PrivacyAction::Anonymize {
                    "[内容已脱敏]".to_string()
                } else {
                    ow.window_title.clone()
                };

                let activity = database::Activity {
                    id: None,
                    timestamp: current_ts,
                    app_name: ow.app_name.clone(),
                    window_title: ow_title,
                    screenshot_path: String::new(),
                    ocr_text: None,
                    category: ow_category,
                    duration: ow_duration,
                    browser_url: None,
                    executable_path: ow.executable_path.clone(),
                    semantic_category: Some(classification.semantic_category),
                    semantic_confidence: Some(i32::from(classification.confidence)),
                };

                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                match state_guard.database.insert_activity(&activity) {
                    Ok(id) => {
                        log::info!(
                            "🪟 浮动窗口新建: {} (id={}, {}s)",
                            ow.app_name,
                            id,
                            ow_duration
                        );
                    }
                    Err(e) => log::error!("浮动窗口记录失败: {e}"),
                }
            }
        }
    }
}

/// 小时摘要生成任务
/// 每小时检查一次，为上一个完整小时生成摘要
/// 为指定日期和小时生成并保存摘要
fn generate_and_save_summary(state: &Arc<Mutex<AppState>>, date: &str, hour: i32) {
    use analysis::hourly::{generate_fallback_summary, HourlyStats};

    let activities = {
        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
        state_guard.database.get_hourly_activities(date, hour)
    };

    match activities {
        Ok(acts) if !acts.is_empty() => {
            let stats = HourlyStats::from_activities(date, hour, acts);
            let summary = generate_fallback_summary(&stats);

            let hourly_summary = database::HourlySummary {
                id: None,
                date: date.to_string(),
                hour,
                summary,
                main_apps: stats.get_main_apps().join(", "),
                activity_count: stats.activity_count,
                total_duration: stats.total_duration,
                representative_screenshots: Some(
                    serde_json::to_string(&stats.representative_screenshots).unwrap_or_default(),
                ),
                created_at: chrono::Local::now().timestamp(),
            };

            let save_result = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard.database.save_hourly_summary(&hourly_summary)
            };

            match save_result {
                Ok(_) => log::info!("小时摘要保存成功: {date} {hour}:00"),
                Err(e) => log::error!("保存小时摘要失败: {e}"),
            }
        }
        Ok(_) => {
            log::debug!("该小时无活动数据: {date} {hour}:00");
        }
        Err(e) => {
            log::error!("获取小时活动数据失败: {e}");
        }
    }
}

async fn hourly_summary_task(state: Arc<Mutex<AppState>>) {
    use chrono::{Local, Timelike};

    // 等待30秒后开始（给应用启动留时间，但不用等太久）
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    // 启动时回填今天所有已过时段的摘要（覆盖旧格式数据）
    {
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let current_hour = now.hour() as i32;

        log::info!("回填今天 0:00 ~ {current_hour}:00 的小时摘要");
        for hour in 0..current_hour {
            generate_and_save_summary(&state, &date, hour);
        }
    }

    loop {
        let now = Local::now();
        let current_hour = now.hour() as i32;
        let date = now.format("%Y-%m-%d").to_string();

        // 为上一个小时生成摘要（如果还没有）
        let target_hour = if current_hour > 0 {
            current_hour - 1
        } else {
            23
        };
        let target_date = if current_hour > 0 {
            date.clone()
        } else {
            (now - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string()
        };

        // 检查是否已有摘要
        let should_generate = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            match state_guard
                .database
                .has_hourly_summary(&target_date, target_hour)
            {
                Ok(has) => !has,
                Err(e) => {
                    log::error!("检查小时摘要失败: {e}");
                    false
                }
            }
        };

        if should_generate {
            log::info!("开始生成 {target_date} {target_hour}:00 的小时摘要");
            generate_and_save_summary(&state, &target_date, target_hour);
        }

        // 休眠到下一个小时的第5分钟
        let next_check = (now + chrono::Duration::hours(1))
            .with_minute(5)
            .unwrap()
            .with_second(0)
            .unwrap();
        let sleep_duration = (next_check - now).num_seconds().max(60) as u64;
        tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
    }
}

#[tauri::command]
fn get_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown";
}

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("work回顾助手启动中...");

    // 获取数据目录
    let data_dir = resolve_data_dir();
    log::info!("数据目录: {data_dir:?}");

    // 加载配置
    let config_path = data_dir.join("config.json");
    let config = AppConfig::load(&config_path).unwrap_or_else(|e| {
        log::warn!("加载配置失败，使用默认配置: {e}");
        AppConfig::default()
    });

    // 初始化数据库
    let db_path = data_dir.join("workreview.db");
    let database = Database::new(&db_path).expect("初始化数据库失败");

    // 初始化隐私过滤器
    let privacy_filter = PrivacyFilter::from_config(&config.privacy);

    // 初始化截屏服务
    let screenshot_service = ScreenshotService::new(&data_dir);

    // macOS: 启动时检查并请求必要的系统权限
    #[cfg(target_os = "macos")]
    {
        // 1. 屏幕录制权限（截图功能必需）
        if !screenshot::has_screen_capture_permission() {
            log::warn!("⚠️  屏幕录制权限未授权，正在请求...");
            log::warn!(
                "   请在「系统设置 → 隐私与安全性 → 屏幕录制」中授权 Work Review，然后重启应用"
            );
            screenshot::request_screen_capture_permission();
        } else {
            log::info!("✅ 屏幕录制权限已授权");
        }

        // 2. 辅助功能权限（读取窗口标题、浏览器 URL 必需）
        if !screenshot::has_accessibility_permission(false) {
            log::warn!("⚠️  辅助功能权限未授权，正在请求...");
            log::warn!("   请在「系统设置 → 隐私与安全性 → 辅助功能」中授权 Work Review");
            // prompt=true 会弹出系统引导对话框
            screenshot::has_accessibility_permission(true);
        } else {
            log::info!("✅ 辅助功能权限已授权");
        }
    }

    // 初始化存储管理器
    let storage_manager = StorageManager::new(&data_dir, config.storage.clone());
    let initial_avatar_opacity = config.avatar_opacity;

    // 启动时执行一次清理
    if let Err(e) = storage_manager.cleanup() {
        log::warn!("启动时清理存储失败: {e}");
    }

    // 创建应用状态，使用 Arc 包装以便在多个地方共享
    let app_state = Arc::new(Mutex::new(AppState {
        config,
        database,
        privacy_filter,
        screenshot_service,
        storage_manager,
        data_dir,
        config_path,
        is_recording: true,
        is_paused: false,
        avatar_state: avatar_engine::apply_avatar_opacity(
            avatar_engine::default_avatar_state(),
            initial_avatar_opacity,
        ),
        avatar_generating_report: false,
    }));
    let app_lifecycle_state = Arc::new(Mutex::new(AppLifecycleState::default()));

    // 构建 Tauri 应用
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        // 开机自启动插件（macOS 使用 LaunchAgent，Windows 使用注册表）
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // 当用户尝试打开第二个实例时，将焦点给到现有窗口
            if let Err(e) = reveal_main_window(&app.clone(), None) {
                log::warn!("恢复主窗口失败: {e}");
            }
            log::info!("检测到重复打开，参数: {argv:?}, 工作目录: {cwd}");
        }))
        .manage(app_state.clone())
        .manage(app_lifecycle_state.clone())
        // 系统托盘在 setup 中创建 (Tauri v2)
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let lightweight_mode = window
                    .try_state::<Arc<Mutex<AppState>>>()
                    .and_then(|state| state.lock().ok().map(|guard| guard.config.lightweight_mode))
                    .unwrap_or(false);

                if main_window_close_behavior(lightweight_mode)
                    == MainWindowCloseBehavior::HideToTray
                {
                    let _ = window.hide();
                    api.prevent_close();
                } else if let Some(lifecycle_state) =
                    window.try_state::<Arc<Mutex<AppLifecycleState>>>()
                {
                    let mut lifecycle_state =
                        lifecycle_state.lock().unwrap_or_else(|e| e.into_inner());
                    lifecycle_state.suppress_next_exit = true;
                }
            } else if let tauri::WindowEvent::Destroyed = event {
                sync_effective_dock_visibility(&window.app_handle());
            }
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            configure_main_window(&window);

            // 获取 Arc<Mutex<AppState>> 并克隆以便在异步任务中使用
            let state = app.state::<Arc<Mutex<AppState>>>();
            let state_clone = state.inner().clone();
            let state_clone2 = state.inner().clone();
            let state_clone3 = state.inner().clone();
            let state_for_tray = state.inner().clone();
            let app_handle = app.handle().clone();
            let screenshot_app_handle = app.handle().clone();

            let (avatar_enabled, avatar_scale, avatar_position, avatar_state) = {
                let state_guard = state.inner().lock().unwrap_or_else(|e| e.into_inner());
                (
                    state_guard.config.avatar_enabled,
                    state_guard.config.avatar_scale,
                    state_guard.config.avatar_x.zip(state_guard.config.avatar_y),
                    state_guard.avatar_state.clone(),
                )
            };

            if let Err(e) = avatar_engine::sync_avatar_window(
                &app.handle(),
                avatar_enabled,
                avatar_scale,
                avatar_position,
            ) {
                log::warn!("初始化桌宠窗口失败: {e}");
            } else if avatar_enabled {
                avatar_engine::emit_avatar_state(&app.handle(), &avatar_state);
            }

            // 创建 Tauri v2 系统托盘
            let show = MenuItemBuilder::with_id(TRAY_MENU_SHOW_ID, "显示窗口").build(app)?;
            let recording_toggle = MenuItemBuilder::with_id(
                TRAY_MENU_RECORDING_TOGGLE_ID,
                tray_recording_toggle_label(true, false),
            )
            .build(app)?;
            let lightweight_mode =
                CheckMenuItemBuilder::with_id(TRAY_MENU_LIGHTWEIGHT_MODE_ID, "轻量模式")
                    .checked(false)
                    .build(app)?;
            let avatar_toggle = CheckMenuItemBuilder::with_id(TRAY_MENU_AVATAR_TOGGLE_ID, "桌宠")
                .checked(avatar_enabled)
                .build(app)?;
            let quit = MenuItemBuilder::with_id(TRAY_MENU_QUIT_ID, "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&show)
                .separator()
                .item(&recording_toggle)
                .item(&lightweight_mode)
                .item(&avatar_toggle)
                .separator()
                .item(&quit)
                .build()?;

            app.manage(TrayMenuState {
                recording_toggle: recording_toggle.clone(),
                lightweight_mode: lightweight_mode.clone(),
                avatar_toggle: avatar_toggle.clone(),
            });
            refresh_tray_menu(&app.handle());

            let tray_icon = build_tray_icon(app);
            let tray_builder = TrayIconBuilder::new().icon(tray_icon).menu(&menu);

            #[cfg(target_os = "macos")]
            let tray_builder = tray_builder.icon_as_template(true);

            let _tray = tray_builder
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    TRAY_MENU_QUIT_ID => {
                        if let Some(lifecycle_state) =
                            app.try_state::<Arc<Mutex<AppLifecycleState>>>()
                        {
                            let mut lifecycle_state =
                                lifecycle_state.lock().unwrap_or_else(|e| e.into_inner());
                            lifecycle_state.explicit_quit_requested = true;
                        }
                        app.exit(0);
                    }
                    TRAY_MENU_SHOW_ID => {
                        if let Err(e) = reveal_main_window(&app.clone(), None) {
                            log::warn!("从托盘恢复主窗口失败: {e}");
                        }
                    }
                    TRAY_MENU_RECORDING_TOGGLE_ID => {
                        {
                            let mut state =
                                state_for_tray.lock().unwrap_or_else(|e| e.into_inner());
                            let action =
                                tray_recording_toggle_action(state.is_recording, state.is_paused);
                            match action {
                                RecordingToggleAction::Start => {
                                    state.is_recording = true;
                                    state.is_paused = false;
                                    log::info!("托盘操作：开始录制");
                                }
                                RecordingToggleAction::Pause => {
                                    state.is_paused = true;
                                    log::info!("托盘操作：暂停录制");
                                }
                                RecordingToggleAction::Resume => {
                                    state.is_paused = false;
                                    log::info!("托盘操作：恢复录制");
                                }
                            }
                        }
                        emit_recording_state_changed(&app);
                    }
                    TRAY_MENU_LIGHTWEIGHT_MODE_ID => {
                        let next_config = {
                            let state = state_for_tray.lock().unwrap_or_else(|e| e.into_inner());
                            let mut config = state.config.clone();
                            config.lightweight_mode = !config.lightweight_mode;
                            config
                        };

                        if let Err(e) =
                            commands::persist_app_config(next_config, app.clone(), &state_for_tray)
                        {
                            log::warn!("从托盘切换轻量模式失败: {e}");
                            refresh_tray_menu(&app);
                        }
                    }
                    TRAY_MENU_AVATAR_TOGGLE_ID => {
                        let next_config = {
                            let state = state_for_tray.lock().unwrap_or_else(|e| e.into_inner());
                            let mut config = state.config.clone();
                            config.avatar_enabled = !config.avatar_enabled;
                            config
                        };

                        if let Err(e) =
                            commands::persist_app_config(next_config, app.clone(), &state_for_tray)
                        {
                            log::warn!("从托盘切换桌宠失败: {e}");
                            refresh_tray_menu(&app);
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |_tray, event| {
                    // 处理托盘图标点击
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = _tray.app_handle();
                        if let Err(e) = reveal_main_window(&app_handle, None) {
                            log::warn!("点击托盘恢复主窗口失败: {e}");
                        }
                    }
                })
                .build(app)?;

            // 启动后台截屏任务
            tauri::async_runtime::spawn(async move {
                background_screenshot_task(state_clone, screenshot_app_handle).await;
            });

            tauri::async_runtime::spawn(async move {
                background_avatar_task(state_clone3, app_handle).await;
            });

            // 启动小时摘要生成任务（每小时检查一次）
            tauri::async_runtime::spawn(async move {
                hourly_summary_task(state_clone2).await;
            });

            // 启动时清理当天的重复记录
            {
                let state_guard = state.inner().lock().unwrap_or_else(|e| e.into_inner());
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                match state_guard.database.cleanup_duplicate_activities(&today) {
                    Ok((deleted, paths)) => {
                        if deleted > 0 {
                            log::warn!("🧹 启动清理: 删除 {deleted} 条重复记录");
                            // 删除对应的截图文件
                            for p in paths {
                                let path = state_guard.data_dir.join(&p);
                                if path.exists() {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("清理重复记录失败: {e}"),
                }

                // decorations 配置由 tauri.conf.json 控制，用户可通过设置中的开关动态修改
            }

            sync_effective_dock_visibility(&app.handle());

            // 保存 AppHandle 到全局变量，用于从 macOS Dock 点击恢复窗口
            let _ = APP_HANDLE.set(app.handle().clone());

            // 注: macOS Dock 点击恢复窗口通过系统托盘 LeftClick 事件处理
            // 用户需要点击状态栏的系统托盘图标来恢复窗口

            log::info!("应用初始化完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_stats,
            commands::get_daily_stats,
            commands::get_timeline,
            commands::generate_report,
            commands::get_saved_report,
            commands::get_config,
            commands::save_config,
            commands::get_update_settings,
            commands::save_update_settings,
            commands::should_check_updates,
            commands::update_last_check_time,
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::get_recording_state,
            commands::get_avatar_state,
            commands::save_avatar_position,
            commands::get_data_dir,
            commands::get_default_data_dir,
            commands::get_runtime_platform,
            commands::change_data_dir,
            commands::check_github_update,
            commands::download_and_install_github_update,
            commands::quit_app_for_update,
            commands::open_data_dir,
            commands::get_screenshot_thumbnail,
            commands::get_screenshot_full,
            commands::take_screenshot,
            commands::test_ai_model,
            commands::test_model,
            commands::get_ai_providers,
            commands::get_running_apps,
            commands::get_recent_apps,
            commands::get_app_category_overview,
            commands::set_app_category_rule,
            commands::reclassify_app_history,
            commands::get_storage_stats,
            commands::get_hourly_summaries,
            commands::get_activity,
            commands::search_memory,
            commands::ask_memory,
            commands::chat_work_assistant,
            commands::get_work_sessions,
            commands::recognize_work_intents,
            commands::generate_weekly_review,
            commands::extract_todo_items,
            commands::clear_old_activities,
            commands::get_ocr_log,
            commands::is_screen_locked,
            commands::check_permissions,
            commands::is_work_time,
            commands::check_ocr_available,
            commands::run_ocr,
            commands::get_ocr_install_guide,
            commands::set_dock_visibility,
            commands::get_app_icon,
            commands::save_background_image,
            commands::get_background_image,
            commands::clear_background_image,
            commands::show_main_window,
            get_platform,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Tauri 应用时出错")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if let Some(lifecycle_state) =
                    _app_handle.try_state::<Arc<Mutex<AppLifecycleState>>>()
                {
                    let mut lifecycle_state =
                        lifecycle_state.lock().unwrap_or_else(|e| e.into_inner());
                    let should_prevent = should_prevent_exit(
                        lifecycle_state.suppress_next_exit,
                        lifecycle_state.explicit_quit_requested,
                    );
                    lifecycle_state.suppress_next_exit = false;

                    if should_prevent {
                        log::info!("拦截最后一个主窗口关闭导致的退出，保留后台与托盘");
                        api.prevent_exit();
                        return;
                    }
                }
            }
            // 处理 macOS Dock 点击：显示隐藏的窗口（仅 macOS）
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } => {
                if !has_visible_windows {
                    if let Err(e) = reveal_main_window(&_app_handle.clone(), None) {
                        log::warn!("Dock 恢复主窗口失败: {e}");
                    }
                }
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::{
        avatar_activity_decision, avatar_monitor_poll_interval_ms, avatar_transition_decision,
        effective_dock_visibility, main_window_close_behavior, monitoring_poll_interval_ms,
        recording_loop_decision, should_prevent_exit, tray_recording_toggle_action,
        tray_recording_toggle_label, MainWindowCloseBehavior, RecordingToggleAction,
    };
    use crate::avatar_engine::{apply_avatar_opacity, default_avatar_state, derive_avatar_state};

    #[test]
    fn 暂停录制时应重置截图计时器() {
        let decision = recording_loop_decision(true, true, 30);
        assert!(!decision.should_continue);
        assert!(decision.reset_capture_clock);
        assert_eq!(decision.screenshot_interval, 1);
    }

    #[test]
    fn 停止录制时应重置截图计时器() {
        let decision = recording_loop_decision(false, false, 30);
        assert!(!decision.should_continue);
        assert!(decision.reset_capture_clock);
        assert_eq!(decision.screenshot_interval, 1);
    }

    #[test]
    fn 正常录制时应保留截图间隔() {
        let decision = recording_loop_decision(true, false, 30);
        assert!(decision.should_continue);
        assert!(!decision.reset_capture_clock);
        assert_eq!(decision.screenshot_interval, 30);
    }

    #[test]
    fn 主监控轮询间隔应保持半秒() {
        assert_eq!(monitoring_poll_interval_ms(), 500);
    }

    #[test]
    fn 桌宠独立轮询间隔应压到一百八十毫秒() {
        assert_eq!(avatar_monitor_poll_interval_ms(), 180);
    }

    #[test]
    fn 暂停录制时桌宠应回到待命状态() {
        let decision = avatar_activity_decision(true, true, true, 0.82);

        assert!(!decision.should_continue);
        assert_eq!(
            decision.reset_state,
            Some(apply_avatar_opacity(default_avatar_state(), 0.82))
        );
    }

    #[test]
    fn 停止录制时桌宠应回到待命状态() {
        let decision = avatar_activity_decision(true, false, false, 0.82);

        assert!(!decision.should_continue);
        assert_eq!(
            decision.reset_state,
            Some(apply_avatar_opacity(default_avatar_state(), 0.82))
        );
    }

    #[test]
    fn 模式首次波动时不应立刻切换桌宠状态() {
        let current = derive_avatar_state("Cursor", "main.rs", None, false, false);
        let candidate = derive_avatar_state("Google Chrome", "产品文档 - docs", None, false, false);

        let decision = avatar_transition_decision(Some(&current), None, 0, &candidate);

        assert_eq!(decision.emit_state, None);
        assert_eq!(decision.pending_state, Some(candidate));
        assert_eq!(decision.pending_hits, 1);
    }

    #[test]
    fn 模式连续两次命中后才应切换桌宠状态() {
        let current = derive_avatar_state("Cursor", "main.rs", None, false, false);
        let candidate = derive_avatar_state("Google Chrome", "产品文档 - docs", None, false, false);

        let decision = avatar_transition_decision(Some(&current), Some(&candidate), 1, &candidate);

        assert_eq!(decision.emit_state, Some(candidate));
        assert_eq!(decision.pending_state, None);
        assert_eq!(decision.pending_hits, 0);
    }

    #[test]
    fn 轻量模式关闭时主窗口关闭按钮应改为隐藏() {
        assert_eq!(
            main_window_close_behavior(false),
            MainWindowCloseBehavior::HideToTray
        );
    }

    #[test]
    fn 轻量模式开启时主窗口关闭按钮应允许真正关闭() {
        assert_eq!(
            main_window_close_behavior(true),
            MainWindowCloseBehavior::CloseWindow
        );
    }

    #[test]
    fn dock可见性应同时考虑用户偏好轻量模式与主窗口是否存在() {
        assert!(!effective_dock_visibility(true, false, true));
        assert!(effective_dock_visibility(false, false, true));
        assert!(effective_dock_visibility(false, true, true));
        assert!(!effective_dock_visibility(false, true, false));
    }

    #[test]
    fn 托盘录制按钮应根据当前状态切换动作() {
        assert_eq!(
            tray_recording_toggle_action(false, false),
            RecordingToggleAction::Start
        );
        assert_eq!(
            tray_recording_toggle_action(true, false),
            RecordingToggleAction::Pause
        );
        assert_eq!(
            tray_recording_toggle_action(true, true),
            RecordingToggleAction::Resume
        );
    }

    #[test]
    fn 托盘录制按钮文案应与状态一致() {
        assert_eq!(tray_recording_toggle_label(false, false), "开始录制");
        assert_eq!(tray_recording_toggle_label(true, false), "暂停录制");
        assert_eq!(tray_recording_toggle_label(true, true), "恢复录制");
    }

    #[test]
    fn 仅应拦截主窗口关闭导致的被动退出() {
        assert!(should_prevent_exit(true, false));
        assert!(!should_prevent_exit(false, false));
        assert!(!should_prevent_exit(true, true));
    }
}
