use crate::avatar_engine::AvatarInputPayload;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const KEYBOARD_ACTIVE_WINDOW_MS: u64 = 180;
const MOUSE_ACTIVE_WINDOW_MS: u64 = 220;

static LAST_KEYBOARD_INPUT_AT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_MOUSE_INPUT_AT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_KEYBOARD_GROUP_CODE: AtomicU8 = AtomicU8::new(0);
static LAST_KEYBOARD_KEY_CODE: AtomicU16 = AtomicU16::new(0);
static LAST_MOUSE_GROUP_CODE: AtomicU8 = AtomicU8::new(0);
static CURSOR_RATIO_X_PERMILLE: AtomicU32 = AtomicU32::new(500);
static CURSOR_RATIO_Y_PERMILLE: AtomicU32 = AtomicU32::new(500);
static INPUT_BRIDGE_STARTED: AtomicBool = AtomicBool::new(false);
static INPUT_MONITOR_STARTED: AtomicBool = AtomicBool::new(false);

const KEYBOARD_GROUP_DIGIT_1: u8 = 1;
const KEYBOARD_GROUP_DIGIT_2: u8 = 2;
const KEYBOARD_GROUP_DIGIT_3: u8 = 3;
const KEYBOARD_GROUP_DIGIT_4: u8 = 4;
const KEYBOARD_GROUP_DIGIT_5: u8 = 5;
const KEYBOARD_GROUP_DIGIT_6: u8 = 6;
const KEYBOARD_GROUP_DIGIT_7: u8 = 7;
const KEYBOARD_GROUP_KEY_Q: u8 = 8;
const KEYBOARD_GROUP_KEY_E: u8 = 9;
const KEYBOARD_GROUP_KEY_R: u8 = 10;
const KEYBOARD_GROUP_SPACE: u8 = 11;
const KEYBOARD_GROUP_KEY_A: u8 = 12;
const KEYBOARD_GROUP_KEY_D: u8 = 13;
const KEYBOARD_GROUP_KEY_S: u8 = 14;
const KEYBOARD_GROUP_KEY_W: u8 = 15;

const MOUSE_GROUP_MOVE: u8 = 1;
const MOUSE_GROUP_LEFT: u8 = 2;
const MOUSE_GROUP_RIGHT: u8 = 3;
const MOUSE_GROUP_SIDE: u8 = 4;
const INPUT_BRIDGE_POLL_INTERVAL: Duration = Duration::from_millis(16);
#[cfg(target_os = "linux")]
const WAYLAND_MOUSE_POLL_INTERVAL: Duration = Duration::from_millis(120);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn is_input_still_active(last_input_at_ms: u64, now_ms: u64, active_window_ms: u64) -> bool {
    last_input_at_ms > 0 && now_ms.saturating_sub(last_input_at_ms) <= active_window_ms
}

fn keyboard_group_label(code: u8) -> &'static str {
    match code {
        KEYBOARD_GROUP_DIGIT_1 => "digit-1",
        KEYBOARD_GROUP_DIGIT_2 => "digit-2",
        KEYBOARD_GROUP_DIGIT_3 => "digit-3",
        KEYBOARD_GROUP_DIGIT_4 => "digit-4",
        KEYBOARD_GROUP_DIGIT_5 => "digit-5",
        KEYBOARD_GROUP_DIGIT_6 => "digit-6",
        KEYBOARD_GROUP_DIGIT_7 => "digit-7",
        KEYBOARD_GROUP_KEY_Q => "key-q",
        KEYBOARD_GROUP_KEY_E => "key-e",
        KEYBOARD_GROUP_KEY_R => "key-r",
        KEYBOARD_GROUP_SPACE => "space",
        KEYBOARD_GROUP_KEY_A => "key-a",
        KEYBOARD_GROUP_KEY_D => "key-d",
        KEYBOARD_GROUP_KEY_S => "key-s",
        KEYBOARD_GROUP_KEY_W => "key-w",
        _ => "idle",
    }
}

fn standard_keyboard_group_from_key_code(key_code: u16) -> u8 {
    match key_code {
        18 => KEYBOARD_GROUP_DIGIT_1,
        19 => KEYBOARD_GROUP_DIGIT_2,
        20 => KEYBOARD_GROUP_DIGIT_3,
        21 => KEYBOARD_GROUP_DIGIT_4,
        23 => KEYBOARD_GROUP_DIGIT_5,
        22 => KEYBOARD_GROUP_DIGIT_6,
        26 => KEYBOARD_GROUP_DIGIT_7,
        12 => KEYBOARD_GROUP_KEY_Q,
        14 => KEYBOARD_GROUP_KEY_E,
        15 => KEYBOARD_GROUP_KEY_R,
        49 => KEYBOARD_GROUP_SPACE,
        0 => KEYBOARD_GROUP_KEY_A,
        2 => KEYBOARD_GROUP_KEY_D,
        1 => KEYBOARD_GROUP_KEY_S,
        13 => KEYBOARD_GROUP_KEY_W,
        _ => 0,
    }
}

fn keyboard_visual_key_from_key_code(key_code: u16) -> &'static str {
    match key_code {
        0 => "KeyA",
        1 => "KeyS",
        2 => "KeyD",
        3 => "KeyF",
        4 => "KeyH",
        5 => "KeyG",
        6 => "KeyZ",
        7 => "KeyX",
        8 => "KeyC",
        9 => "KeyV",
        11 => "KeyB",
        12 => "KeyQ",
        13 => "KeyW",
        14 => "KeyE",
        15 => "KeyR",
        16 => "KeyY",
        17 => "KeyT",
        18 => "Num1",
        19 => "Num2",
        20 => "Num3",
        21 => "Num4",
        22 => "Num6",
        23 => "Num5",
        25 => "Num9",
        26 => "Num7",
        28 => "Num8",
        29 => "Num0",
        31 => "KeyO",
        32 => "KeyU",
        34 => "KeyI",
        35 => "KeyP",
        36 => "Return",
        37 => "KeyL",
        38 => "KeyJ",
        40 => "KeyK",
        43 => "Comma",
        44 => "Slash",
        45 => "KeyN",
        46 => "KeyM",
        47 => "Period",
        48 => "Tab",
        49 => "Space",
        50 => "BackQuote",
        51 => "Backspace",
        53 => "Escape",
        55 => "Meta",
        56 => "ShiftLeft",
        57 => "CapsLock",
        58 => "Alt",
        59 => "ControlLeft",
        60 => "ShiftRight",
        61 => "AltGr",
        62 => "ControlRight",
        63 => "Fn",
        117 => "Delete",
        123 => "LeftArrow",
        124 => "RightArrow",
        125 => "DownArrow",
        126 => "UpArrow",
        _ => "",
    }
}

fn mouse_group_label(code: u8) -> &'static str {
    match code {
        MOUSE_GROUP_LEFT => "mouse-left",
        MOUSE_GROUP_RIGHT => "mouse-right",
        MOUSE_GROUP_SIDE => "mouse-side",
        MOUSE_GROUP_MOVE => "mouse-move",
        _ => "idle",
    }
}

fn linux_session_supports_avatar_input(session: crate::linux_session::LinuxDesktopSession) -> bool {
    matches!(session, crate::linux_session::LinuxDesktopSession::X11)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxAvatarInputSupport {
    pub provider: &'static str,
    pub support_level: &'static str,
    pub keyboard_supported: bool,
    pub mouse_supported: bool,
}

impl LinuxAvatarInputSupport {
    const fn full(provider: &'static str) -> Self {
        Self {
            provider,
            support_level: "full",
            keyboard_supported: true,
            mouse_supported: true,
        }
    }

    const fn mouse_only(provider: &'static str) -> Self {
        Self {
            provider,
            support_level: "mouse-only",
            keyboard_supported: false,
            mouse_supported: true,
        }
    }

    const fn none() -> Self {
        Self {
            provider: "none",
            support_level: "none",
            keyboard_supported: false,
            mouse_supported: false,
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinuxWaylandMouseProvider {
    GnomeShellDbus,
    KdeKdotool,
    HyprlandHyprctl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LinuxWaylandMouseSnapshot {
    x: i32,
    y: i32,
    group_code: u8,
    keyboard_key_code: Option<u16>,
    keyboard_timestamp_ms: Option<u64>,
}

fn linux_avatar_input_support_for_session(
    session: crate::linux_session::LinuxDesktopSession,
    desktop_environment: crate::linux_session::LinuxDesktopEnvironment,
    gnome_mouse_provider_available: bool,
    kde_mouse_provider_available: bool,
    hyprland_mouse_provider_available: bool,
) -> LinuxAvatarInputSupport {
    use crate::linux_session::{LinuxDesktopEnvironment, LinuxDesktopSession};

    match session {
        LinuxDesktopSession::X11 => LinuxAvatarInputSupport::full("xinput2"),
        LinuxDesktopSession::Wayland => match desktop_environment {
            LinuxDesktopEnvironment::Gnome if gnome_mouse_provider_available => {
                LinuxAvatarInputSupport::full("gnome-shell-dbus")
            }
            LinuxDesktopEnvironment::Kde if kde_mouse_provider_available => {
                LinuxAvatarInputSupport::mouse_only("kdotool-mouselocation")
            }
            LinuxDesktopEnvironment::Hyprland if hyprland_mouse_provider_available => {
                LinuxAvatarInputSupport::mouse_only("hyprctl-cursorpos")
            }
            LinuxDesktopEnvironment::Unknown if gnome_mouse_provider_available => {
                LinuxAvatarInputSupport::full("gnome-shell-dbus")
            }
            LinuxDesktopEnvironment::Unknown if hyprland_mouse_provider_available => {
                LinuxAvatarInputSupport::mouse_only("hyprctl-cursorpos")
            }
            LinuxDesktopEnvironment::Unknown if kde_mouse_provider_available => {
                LinuxAvatarInputSupport::mouse_only("kdotool-mouselocation")
            }
            _ => LinuxAvatarInputSupport::none(),
        },
        LinuxDesktopSession::Unknown => LinuxAvatarInputSupport::none(),
    }
}

fn linux_keysym_to_avatar_key_code(keysym: u64) -> Option<u16> {
    match keysym {
        0x0030 => Some(29),
        0x0031 => Some(18),
        0x0032 => Some(19),
        0x0033 => Some(20),
        0x0034 => Some(21),
        0x0035 => Some(23),
        0x0036 => Some(22),
        0x0037 => Some(26),
        0x0038 => Some(28),
        0x0039 => Some(25),
        0x0041 | 0x0061 => Some(0),
        0x0042 | 0x0062 => Some(11),
        0x0043 | 0x0063 => Some(8),
        0x0044 | 0x0064 => Some(2),
        0x0045 | 0x0065 => Some(14),
        0x0046 | 0x0066 => Some(3),
        0x0047 | 0x0067 => Some(5),
        0x0048 | 0x0068 => Some(4),
        0x0049 | 0x0069 => Some(34),
        0x004A | 0x006A => Some(38),
        0x004B | 0x006B => Some(40),
        0x004C | 0x006C => Some(37),
        0x004D | 0x006D => Some(46),
        0x004E | 0x006E => Some(45),
        0x004F | 0x006F => Some(31),
        0x0050 | 0x0070 => Some(35),
        0x0051 | 0x0071 => Some(12),
        0x0052 | 0x0072 => Some(15),
        0x0053 | 0x0073 => Some(1),
        0x0054 | 0x0074 => Some(17),
        0x0055 | 0x0075 => Some(32),
        0x0056 | 0x0076 => Some(9),
        0x0057 | 0x0077 => Some(13),
        0x0058 | 0x0078 => Some(7),
        0x0059 | 0x0079 => Some(16),
        0x005A | 0x007A => Some(6),
        0x0020 => Some(49),
        0x002C => Some(43),
        0x002E => Some(47),
        0x002F => Some(44),
        0x0060 => Some(50),
        0xFF08 => Some(51),
        0xFF09 => Some(48),
        0xFF0D => Some(36),
        0xFF1B => Some(53),
        0xFF51 => Some(123),
        0xFF52 => Some(126),
        0xFF53 => Some(124),
        0xFF54 => Some(125),
        0xFFFF => Some(117),
        0xFFE1 => Some(56),
        0xFFE2 => Some(60),
        0xFFE3 => Some(59),
        0xFFE4 => Some(62),
        0xFFE5 => Some(57),
        0xFFE7 | 0xFFE8 | 0xFFEB | 0xFFEC => Some(55),
        0xFFE9 => Some(58),
        0xFFEA => Some(61),
        _ => None,
    }
}

fn linux_mouse_group_from_button_detail(detail: i32) -> u8 {
    match detail {
        1 => MOUSE_GROUP_LEFT,
        3 => MOUSE_GROUP_RIGHT,
        2 | 8 | 9 => MOUSE_GROUP_SIDE,
        _ => MOUSE_GROUP_MOVE,
    }
}

fn mouse_group_code_from_label(value: &str) -> u8 {
    match value.trim() {
        "mouse-left" => MOUSE_GROUP_LEFT,
        "mouse-right" => MOUSE_GROUP_RIGHT,
        "mouse-side" => MOUSE_GROUP_SIDE,
        "mouse-move" => MOUSE_GROUP_MOVE,
        _ => MOUSE_GROUP_MOVE,
    }
}

fn parse_gnome_avatar_input_dbus_output(output: &str) -> Option<LinuxWaylandMouseSnapshot> {
    let json_start = output.find('{')?;
    let json_end = output.rfind('}')?;
    let payload = &output[json_start..=json_end];
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;

    let x = value.get("x")?.as_i64()? as i32;
    let y = value.get("y")?.as_i64()? as i32;
    let group_code = value
        .get("mouseGroup")
        .and_then(|entry| entry.as_str())
        .map(mouse_group_code_from_label)
        .unwrap_or(MOUSE_GROUP_MOVE);
    let keyboard_key_code = value
        .get("keyval")
        .and_then(|entry| entry.as_u64())
        .and_then(linux_keysym_to_avatar_key_code);
    let keyboard_timestamp_ms = value
        .get("keyboardTimestampMs")
        .and_then(|entry| entry.as_u64());

    Some(LinuxWaylandMouseSnapshot {
        x,
        y,
        group_code,
        keyboard_key_code,
        keyboard_timestamp_ms,
    })
}

fn parse_kdotool_mouse_location_output(output: &str) -> Option<(i32, i32)> {
    let mut x = None;
    let mut y = None;

    for segment in output.split_whitespace() {
        if let Some(value) = segment.strip_prefix("x:") {
            x = value.parse::<i32>().ok();
        } else if let Some(value) = segment.strip_prefix("y:") {
            y = value.parse::<i32>().ok();
        }
    }

    Some((x?, y?))
}

fn parse_hyprctl_cursorpos_output(output: &str) -> Option<(i32, i32)> {
    let normalized = output.trim().replace(',', " ");
    let mut parts = normalized.split_whitespace();
    let x = parts.next()?.parse::<i32>().ok()?;
    let y = parts.next()?.parse::<i32>().ok()?;
    Some((x, y))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn cursor_ratio_from_virtual_desktop_bounds(
    point_x: i32,
    point_y: i32,
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
) -> (f64, f64) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    (
        ((point_x - min_x) as f64 / width).clamp(0.0, 1.0),
        ((point_y - min_y) as f64 / height).clamp(0.0, 1.0),
    )
}

#[cfg(target_os = "linux")]
fn run_linux_avatar_command_with_timeout(
    command: &mut std::process::Command,
    context: &str,
) -> Option<std::process::Output> {
    use std::process::Stdio;
    use std::thread;
    use std::time::Instant;

    let timeout = Duration::from_millis(1000);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command.spawn().ok()?;
    let started_at = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if started_at.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                log::debug!("桌宠输入联动命令超时：{context}");
                return None;
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                log::debug!("桌宠输入联动命令执行失败（{context}）：{error}");
                return None;
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn is_kde_wayland_mouse_provider_available() -> bool {
    run_linux_avatar_command_with_timeout(
        std::process::Command::new("kdotool").arg("getmouselocation"),
        "kdotool getmouselocation",
    )
    .and_then(|output| {
        parse_kdotool_mouse_location_output(&String::from_utf8_lossy(&output.stdout))
    })
    .is_some()
}

#[cfg(target_os = "linux")]
fn is_gnome_wayland_mouse_provider_available() -> bool {
    run_linux_avatar_command_with_timeout(
        std::process::Command::new("gdbus").args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/gnome/shell/extensions/WorkReviewAvatarInput",
            "--method",
            "org.gnome.shell.extensions.WorkReviewAvatarInput.GetInput",
        ]),
        "gdbus WorkReviewAvatarInput.GetInput",
    )
    .filter(|output| output.status.success())
    .and_then(|output| {
        parse_gnome_avatar_input_dbus_output(&String::from_utf8_lossy(&output.stdout))
    })
    .is_some()
}

#[cfg(target_os = "linux")]
fn is_hyprland_wayland_mouse_provider_available() -> bool {
    run_linux_avatar_command_with_timeout(
        std::process::Command::new("hyprctl").arg("cursorpos"),
        "hyprctl cursorpos",
    )
    .and_then(|output| parse_hyprctl_cursorpos_output(&String::from_utf8_lossy(&output.stdout)))
    .is_some()
}

#[cfg(target_os = "linux")]
fn linux_wayland_mouse_provider_for_environment(
    desktop_environment: crate::linux_session::LinuxDesktopEnvironment,
    gnome_mouse_provider_available: bool,
    kde_mouse_provider_available: bool,
    hyprland_mouse_provider_available: bool,
) -> Option<LinuxWaylandMouseProvider> {
    use crate::linux_session::LinuxDesktopEnvironment;

    match desktop_environment {
        LinuxDesktopEnvironment::Gnome if gnome_mouse_provider_available => {
            Some(LinuxWaylandMouseProvider::GnomeShellDbus)
        }
        LinuxDesktopEnvironment::Kde if kde_mouse_provider_available => {
            Some(LinuxWaylandMouseProvider::KdeKdotool)
        }
        LinuxDesktopEnvironment::Hyprland if hyprland_mouse_provider_available => {
            Some(LinuxWaylandMouseProvider::HyprlandHyprctl)
        }
        LinuxDesktopEnvironment::Unknown if gnome_mouse_provider_available => {
            Some(LinuxWaylandMouseProvider::GnomeShellDbus)
        }
        LinuxDesktopEnvironment::Unknown if hyprland_mouse_provider_available => {
            Some(LinuxWaylandMouseProvider::HyprlandHyprctl)
        }
        LinuxDesktopEnvironment::Unknown if kde_mouse_provider_available => {
            Some(LinuxWaylandMouseProvider::KdeKdotool)
        }
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn query_wayland_mouse_cursor_point(
    provider: LinuxWaylandMouseProvider,
) -> Option<LinuxWaylandMouseSnapshot> {
    match provider {
        LinuxWaylandMouseProvider::GnomeShellDbus => run_linux_avatar_command_with_timeout(
            std::process::Command::new("gdbus").args([
                "call",
                "--session",
                "--dest",
                "org.gnome.Shell",
                "--object-path",
                "/org/gnome/shell/extensions/WorkReviewAvatarInput",
                "--method",
                "org.gnome.shell.extensions.WorkReviewAvatarInput.GetInput",
            ]),
            "gdbus WorkReviewAvatarInput.GetInput",
        )
        .and_then(|output| {
            parse_gnome_avatar_input_dbus_output(&String::from_utf8_lossy(&output.stdout))
        }),
        LinuxWaylandMouseProvider::KdeKdotool => run_linux_avatar_command_with_timeout(
            std::process::Command::new("kdotool").arg("getmouselocation"),
            "kdotool getmouselocation",
        )
        .and_then(|output| {
            parse_kdotool_mouse_location_output(&String::from_utf8_lossy(&output.stdout)).map(
                |(x, y)| LinuxWaylandMouseSnapshot {
                    x,
                    y,
                    group_code: MOUSE_GROUP_MOVE,
                    keyboard_key_code: None,
                    keyboard_timestamp_ms: None,
                },
            )
        }),
        LinuxWaylandMouseProvider::HyprlandHyprctl => run_linux_avatar_command_with_timeout(
            std::process::Command::new("hyprctl").arg("cursorpos"),
            "hyprctl cursorpos",
        )
        .and_then(|output| {
            parse_hyprctl_cursorpos_output(&String::from_utf8_lossy(&output.stdout)).map(
                |(x, y)| LinuxWaylandMouseSnapshot {
                    x,
                    y,
                    group_code: MOUSE_GROUP_MOVE,
                    keyboard_key_code: None,
                    keyboard_timestamp_ms: None,
                },
            )
        }),
    }
}

#[cfg(target_os = "linux")]
fn virtual_desktop_bounds_from_monitors(app: &AppHandle) -> Option<(i32, i32, u32, u32)> {
    let monitors = app.available_monitors().ok()?;
    let first = monitors.first()?;
    let mut min_x = first.position().x;
    let mut min_y = first.position().y;
    let mut max_x = first.position().x + first.size().width as i32;
    let mut max_y = first.position().y + first.size().height as i32;

    for monitor in monitors.iter().skip(1) {
        let position = monitor.position();
        let size = monitor.size();
        min_x = min_x.min(position.x);
        min_y = min_y.min(position.y);
        max_x = max_x.max(position.x + size.width as i32);
        max_y = max_y.max(position.y + size.height as i32);
    }

    let width = max_x.checked_sub(min_x)? as u32;
    let height = max_y.checked_sub(min_y)? as u32;
    Some((min_x, min_y, width, height))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[cfg(target_os = "linux")]
pub fn current_linux_avatar_input_support() -> LinuxAvatarInputSupport {
    let session = crate::linux_session::current_linux_desktop_session();
    let desktop_environment = crate::linux_session::current_linux_desktop_environment();
    linux_avatar_input_support_for_session(
        session,
        desktop_environment,
        is_gnome_wayland_mouse_provider_available(),
        is_kde_wayland_mouse_provider_available(),
        is_hyprland_wayland_mouse_provider_available(),
    )
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[cfg(not(target_os = "linux"))]
pub fn current_linux_avatar_input_support() -> LinuxAvatarInputSupport {
    LinuxAvatarInputSupport::none()
}

#[cfg(target_os = "macos")]
fn mouse_group_from_event_type(event_type: cocoa::appkit::NSEventType) -> u8 {
    use cocoa::appkit::NSEventType;

    match event_type {
        NSEventType::NSLeftMouseDown => MOUSE_GROUP_LEFT,
        NSEventType::NSRightMouseDown => MOUSE_GROUP_RIGHT,
        NSEventType::NSOtherMouseDown => MOUSE_GROUP_SIDE,
        _ => MOUSE_GROUP_MOVE,
    }
}

#[cfg(target_os = "windows")]
fn windows_virtual_key_to_avatar_key_code(virtual_key: u32) -> Option<u16> {
    match virtual_key {
        0x30 => Some(29),
        0x31 => Some(18),
        0x32 => Some(19),
        0x33 => Some(20),
        0x34 => Some(21),
        0x35 => Some(23),
        0x36 => Some(22),
        0x37 => Some(26),
        0x38 => Some(28),
        0x39 => Some(25),
        0x41 => Some(0),
        0x42 => Some(11),
        0x43 => Some(8),
        0x44 => Some(2),
        0x45 => Some(14),
        0x46 => Some(3),
        0x47 => Some(5),
        0x48 => Some(4),
        0x49 => Some(34),
        0x4A => Some(38),
        0x4B => Some(40),
        0x4C => Some(37),
        0x4D => Some(46),
        0x4E => Some(45),
        0x4F => Some(31),
        0x50 => Some(35),
        0x51 => Some(12),
        0x52 => Some(15),
        0x53 => Some(1),
        0x54 => Some(17),
        0x55 => Some(32),
        0x56 => Some(9),
        0x57 => Some(13),
        0x58 => Some(7),
        0x59 => Some(16),
        0x5A => Some(6),
        0x08 => Some(51),
        0x09 => Some(48),
        0x0D => Some(36),
        0x14 => Some(57),
        0x1B => Some(53),
        0x20 => Some(49),
        0x25 => Some(123),
        0x26 => Some(126),
        0x27 => Some(124),
        0x28 => Some(125),
        0x2C => Some(43),
        0x2E => Some(47),
        0x2F => Some(44),
        0x2D => Some(117),
        0xA0 => Some(56),
        0xA1 => Some(60),
        0xA2 => Some(59),
        0xA3 => Some(62),
        0xA4 => Some(58),
        0xA5 => Some(61),
        0x5B | 0x5C => Some(55),
        0xC0 => Some(50),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn windows_mouse_group_from_message(message: u32) -> u8 {
    use winapi::um::winuser::{
        WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
        WM_RBUTTONDOWN, WM_XBUTTONDOWN,
    };

    match message {
        WM_LBUTTONDOWN => MOUSE_GROUP_LEFT,
        WM_RBUTTONDOWN => MOUSE_GROUP_RIGHT,
        WM_MBUTTONDOWN | WM_XBUTTONDOWN => MOUSE_GROUP_SIDE,
        WM_MOUSEMOVE | WM_MOUSEWHEEL | WM_MOUSEHWHEEL => MOUSE_GROUP_MOVE,
        _ => MOUSE_GROUP_MOVE,
    }
}

#[cfg(target_os = "windows")]
fn cursor_ratio_from_virtual_screen(point_x: i32, point_y: i32) -> (f64, f64) {
    use winapi::um::winuser::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    let min_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let min_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) }.max(1) as f64;
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) }.max(1) as f64;
    let x_ratio = ((point_x - min_x) as f64 / width).clamp(0.0, 1.0);
    let y_ratio = ((point_y - min_y) as f64 / height).clamp(0.0, 1.0);

    (x_ratio, y_ratio)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn windows_keyboard_hook_proc(
    code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    use std::ptr;
    use winapi::um::winuser::{CallNextHookEx, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN};

    if code >= 0 && (w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN) {
        let keyboard_info = &*(l_param as *const KBDLLHOOKSTRUCT);
        if let Some(key_code) = windows_virtual_key_to_avatar_key_code(keyboard_info.vkCode) {
            record_keyboard_input(standard_keyboard_group_from_key_code(key_code), key_code);
        }
    }

    CallNextHookEx(ptr::null_mut(), code, w_param, l_param)
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn windows_mouse_hook_proc(
    code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    use std::ptr;
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::{
        CallNextHookEx, GetCursorPos, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
        WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
    };

    let message = w_param as u32;
    let tracked_message = matches!(
        message,
        WM_MOUSEMOVE
            | WM_MOUSEWHEEL
            | WM_MOUSEHWHEEL
            | WM_LBUTTONDOWN
            | WM_RBUTTONDOWN
            | WM_MBUTTONDOWN
            | WM_XBUTTONDOWN
    );

    if code >= 0 && tracked_message {
        let mouse_info = &*(l_param as *const MSLLHOOKSTRUCT);
        record_mouse_input(windows_mouse_group_from_message(message));
        let mut point = POINT {
            x: mouse_info.pt.x,
            y: mouse_info.pt.y,
        };
        if GetCursorPos(&mut point) == 0 {
            point = mouse_info.pt;
        }
        let (cursor_ratio_x, cursor_ratio_y) = cursor_ratio_from_virtual_screen(point.x, point.y);
        record_cursor_ratio(cursor_ratio_x, cursor_ratio_y);
    }

    CallNextHookEx(ptr::null_mut(), code, w_param, l_param)
}

#[cfg(target_os = "linux")]
unsafe fn linux_query_cursor_ratio(
    display: *mut x11::xlib::Display,
    root_window: x11::xlib::Window,
    screen: i32,
) -> Option<(f64, f64)> {
    use std::mem;
    use x11::xlib;

    let mut root_return: x11::xlib::Window = 0;
    let mut child_return: x11::xlib::Window = 0;
    let mut root_x = 0;
    let mut root_y = 0;
    let mut win_x = 0;
    let mut win_y = 0;
    let mut mask_return: u32 = mem::zeroed();

    if xlib::XQueryPointer(
        display,
        root_window,
        &mut root_return,
        &mut child_return,
        &mut root_x,
        &mut root_y,
        &mut win_x,
        &mut win_y,
        &mut mask_return,
    ) == xlib::False
    {
        return None;
    }

    let width = xlib::XDisplayWidth(display, screen).max(1) as f64;
    let height = xlib::XDisplayHeight(display, screen).max(1) as f64;

    Some((
        (root_x as f64 / width).clamp(0.0, 1.0),
        (root_y as f64 / height).clamp(0.0, 1.0),
    ))
}

pub(crate) fn build_avatar_input_payload(now_ms: u64) -> AvatarInputPayload {
    let last_keyboard_input_at_ms = LAST_KEYBOARD_INPUT_AT_MS.load(Ordering::Relaxed);
    let last_mouse_input_at_ms = LAST_MOUSE_INPUT_AT_MS.load(Ordering::Relaxed);
    let keyboard_group_code = LAST_KEYBOARD_GROUP_CODE.load(Ordering::Relaxed);
    let keyboard_key_code = LAST_KEYBOARD_KEY_CODE.load(Ordering::Relaxed);
    let mouse_group_code = LAST_MOUSE_GROUP_CODE.load(Ordering::Relaxed);

    AvatarInputPayload {
        keyboard_active: is_input_still_active(
            last_keyboard_input_at_ms,
            now_ms,
            KEYBOARD_ACTIVE_WINDOW_MS,
        ),
        mouse_active: is_input_still_active(last_mouse_input_at_ms, now_ms, MOUSE_ACTIVE_WINDOW_MS),
        keyboard_group: keyboard_group_label(keyboard_group_code).to_string(),
        keyboard_visual_key: keyboard_visual_key_from_key_code(keyboard_key_code).to_string(),
        mouse_group: mouse_group_label(mouse_group_code).to_string(),
        cursor_ratio_x: CURSOR_RATIO_X_PERMILLE.load(Ordering::Relaxed) as f64 / 1000.0,
        cursor_ratio_y: CURSOR_RATIO_Y_PERMILLE.load(Ordering::Relaxed) as f64 / 1000.0,
        last_keyboard_input_at_ms,
        last_mouse_input_at_ms,
    }
}

pub(crate) fn record_keyboard_input(group_code: u8, key_code: u16) {
    LAST_KEYBOARD_INPUT_AT_MS.store(now_ms(), Ordering::Relaxed);
    LAST_KEYBOARD_GROUP_CODE.store(group_code, Ordering::Relaxed);
    LAST_KEYBOARD_KEY_CODE.store(key_code, Ordering::Relaxed);
}

pub(crate) fn record_mouse_input(group_code: u8) {
    LAST_MOUSE_INPUT_AT_MS.store(now_ms(), Ordering::Relaxed);
    LAST_MOUSE_GROUP_CODE.store(group_code, Ordering::Relaxed);
}

pub(crate) fn record_cursor_ratio(x_ratio: f64, y_ratio: f64) {
    let to_permille = |value: f64| -> u32 { (value.clamp(0.0, 1.0) * 1000.0).round() as u32 };

    CURSOR_RATIO_X_PERMILLE.store(to_permille(x_ratio), Ordering::Relaxed);
    CURSOR_RATIO_Y_PERMILLE.store(to_permille(y_ratio), Ordering::Relaxed);
}

pub fn spawn_avatar_input_bridge(app: AppHandle) {
    if INPUT_BRIDGE_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    tauri::async_runtime::spawn(async move {
        let mut last_payload: Option<AvatarInputPayload> = None;

        loop {
            let next_payload = build_avatar_input_payload(now_ms());
            if last_payload.as_ref() != Some(&next_payload) {
                crate::avatar_engine::emit_avatar_input(&app, &next_payload);
                last_payload = Some(next_payload);
            }

            tokio::time::sleep(INPUT_BRIDGE_POLL_INTERVAL).await;
        }
    });
}

#[cfg(target_os = "macos")]
thread_local! {
    static MACOS_INPUT_MONITOR: std::cell::RefCell<Option<MacosInputMonitor>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
struct MacosInputMonitor {
    keyboard_monitor: cocoa::base::id,
    mouse_monitor: cocoa::base::id,
    keyboard_handler: block::RcBlock<(cocoa::base::id,), ()>,
    mouse_handler: block::RcBlock<(cocoa::base::id,), ()>,
}

#[cfg(target_os = "macos")]
pub fn start_avatar_input_monitor(app: &AppHandle) {
    use block::ConcreteBlock;
    use cocoa::appkit::{NSEventMask, NSEventType};
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSPoint, NSRect};

    unsafe fn current_cursor_ratio() -> (f64, f64) {
        let screens: id = msg_send![class!(NSScreen), screens];
        let count: usize = msg_send![screens, count];
        if count == 0 {
            return (0.5, 0.5);
        }

        let mut min_x = 0.0;
        let mut min_y = 0.0;
        let mut max_x = 0.0;
        let mut max_y = 0.0;

        for index in 0..count {
            let screen: id = msg_send![screens, objectAtIndex: index];
            let frame: NSRect = msg_send![screen, frame];
            let left = frame.origin.x;
            let bottom = frame.origin.y;
            let right = frame.origin.x + frame.size.width;
            let top = frame.origin.y + frame.size.height;

            if index == 0 {
                min_x = left;
                min_y = bottom;
                max_x = right;
                max_y = top;
            } else {
                min_x = min_x.min(left);
                min_y = min_y.min(bottom);
                max_x = max_x.max(right);
                max_y = max_y.max(top);
            }
        }

        let point: NSPoint = msg_send![class!(NSEvent), mouseLocation];
        let width = (max_x - min_x).max(1.0);
        let height = (max_y - min_y).max(1.0);
        let x_ratio = ((point.x - min_x) / width).clamp(0.0, 1.0);
        let y_ratio = (1.0 - ((point.y - min_y) / height)).clamp(0.0, 1.0);

        (x_ratio, y_ratio)
    }

    if INPUT_MONITOR_STARTED.load(Ordering::SeqCst) {
        return;
    }

    if !crate::screenshot::has_accessibility_permission(false) {
        log::warn!("桌宠输入联动未启动：缺少辅助功能权限");
        return;
    }

    if !crate::screenshot::has_input_monitoring_permission() {
        log::warn!("桌宠输入联动未启动：缺少输入监控权限");
        return;
    }

    if INPUT_MONITOR_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let run_result = app.run_on_main_thread(move || unsafe {
        MACOS_INPUT_MONITOR.with(|slot| {
            if slot.borrow().is_some() {
                return;
            }

            let event_class = class!(NSEvent);
            let keyboard_handler = ConcreteBlock::new(|event: id| {
                let key_code: u16 = msg_send![event, keyCode];
                record_keyboard_input(standard_keyboard_group_from_key_code(key_code), key_code);
            })
            .copy();
            let mouse_handler = ConcreteBlock::new(|event: id| {
                let event_type_raw: usize = msg_send![event, type];
                let event_type = std::mem::transmute::<usize, NSEventType>(event_type_raw);
                record_mouse_input(mouse_group_from_event_type(event_type));
                let (cursor_ratio_x, cursor_ratio_y) = current_cursor_ratio();
                record_cursor_ratio(cursor_ratio_x, cursor_ratio_y);
            })
            .copy();

            let keyboard_mask =
                (NSEventMask::NSKeyDownMask | NSEventMask::NSFlagsChangedMask).bits();
            let mouse_mask = (NSEventMask::NSLeftMouseDownMask
                | NSEventMask::NSRightMouseDownMask
                | NSEventMask::NSOtherMouseDownMask
                | NSEventMask::NSMouseMovedMask
                | NSEventMask::NSScrollWheelMask)
                .bits();

            let keyboard_monitor: id = msg_send![
                event_class,
                addGlobalMonitorForEventsMatchingMask: keyboard_mask
                handler: &*keyboard_handler
            ];
            let mouse_monitor: id = msg_send![
                event_class,
                addGlobalMonitorForEventsMatchingMask: mouse_mask
                handler: &*mouse_handler
            ];

            if keyboard_monitor == nil || mouse_monitor == nil {
                log::warn!("桌宠输入联动注册失败：系统未返回有效的全局监听句柄");
                return;
            }

            slot.replace(Some(MacosInputMonitor {
                keyboard_monitor,
                mouse_monitor,
                keyboard_handler,
                mouse_handler,
            }));
        });
    });

    if let Err(e) = run_result {
        INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
        log::warn!("桌宠输入联动注册失败: {e}");
    }
}

#[cfg(target_os = "windows")]
pub fn start_avatar_input_monitor(_app: &AppHandle) {
    use std::{mem, ptr, thread};
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::winuser::{
        DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
        MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
    };

    if INPUT_MONITOR_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    thread::spawn(|| unsafe {
        let module_handle = GetModuleHandleW(ptr::null());
        let keyboard_hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(windows_keyboard_hook_proc),
            module_handle,
            0,
        );
        let mouse_hook =
            SetWindowsHookExW(WH_MOUSE_LL, Some(windows_mouse_hook_proc), module_handle, 0);

        if keyboard_hook.is_null() || mouse_hook.is_null() {
            if !keyboard_hook.is_null() {
                UnhookWindowsHookEx(keyboard_hook);
            }
            if !mouse_hook.is_null() {
                UnhookWindowsHookEx(mouse_hook);
            }
            INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
            log::warn!("桌宠输入联动注册失败：Windows 低级键鼠 Hook 初始化失败");
            return;
        }

        let mut message: MSG = mem::zeroed();
        loop {
            let result = GetMessageW(&mut message, ptr::null_mut(), 0, 0);
            if result == -1 {
                log::warn!("桌宠输入联动消息循环异常退出：Windows 消息泵返回错误");
                break;
            }
            if result == 0 {
                break;
            }

            TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        UnhookWindowsHookEx(keyboard_hook);
        UnhookWindowsHookEx(mouse_hook);
        INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
    });
}

#[cfg(target_os = "linux")]
pub fn start_avatar_input_monitor(app: &AppHandle) {
    use crate::linux_session::{
        current_linux_desktop_environment, current_linux_desktop_session, LinuxDesktopSession,
    };
    use std::{ffi::CString, mem, ptr, thread};
    use x11::{xinput2, xlib};

    let session = current_linux_desktop_session();
    let desktop_environment = current_linux_desktop_environment();
    let gnome_mouse_provider_available = is_gnome_wayland_mouse_provider_available();
    let kde_mouse_provider_available = is_kde_wayland_mouse_provider_available();
    let hyprland_mouse_provider_available = is_hyprland_wayland_mouse_provider_available();
    let support = linux_avatar_input_support_for_session(
        session,
        desktop_environment,
        gnome_mouse_provider_available,
        kde_mouse_provider_available,
        hyprland_mouse_provider_available,
    );

    if support.support_level == "none" {
        log::warn!(
            "桌宠输入联动未启动：Linux {} / {} 会话暂不支持可用的桌宠输入 provider",
            session.as_str(),
            desktop_environment.as_str()
        );
        return;
    }

    if INPUT_MONITOR_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    match session {
        LinuxDesktopSession::X11 => {
            thread::spawn(|| unsafe {
                xlib::XInitThreads();

                let display = xlib::XOpenDisplay(ptr::null());
                if display.is_null() {
                    INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
                    log::warn!("桌宠输入联动注册失败：无法连接到 X11 Display");
                    return;
                }

                let screen = xlib::XDefaultScreen(display);
                let root_window = xlib::XRootWindow(display, screen);

                let mut opcode = 0;
                let mut first_event = 0;
                let mut first_error = 0;
                let xinput_extension = CString::new("XInputExtension").expect("固定字符串不应失败");
                if xlib::XQueryExtension(
                    display,
                    xinput_extension.as_ptr(),
                    &mut opcode,
                    &mut first_event,
                    &mut first_error,
                ) == xlib::False
                {
                    xlib::XCloseDisplay(display);
                    INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
                    log::warn!("桌宠输入联动注册失败：XInputExtension 不可用");
                    return;
                }

                let mut major = xinput2::XI_2_Major;
                let mut minor = xinput2::XI_2_Minor;
                if xinput2::XIQueryVersion(display, &mut major, &mut minor) != xlib::Success as i32
                {
                    xlib::XCloseDisplay(display);
                    INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
                    log::warn!("桌宠输入联动注册失败：XInput2 不可用");
                    return;
                }

                let mut mask = [0_u8; 4];
                xinput2::XISetMask(&mut mask, xinput2::XI_RawKeyPress);
                xinput2::XISetMask(&mut mask, xinput2::XI_RawButtonPress);
                xinput2::XISetMask(&mut mask, xinput2::XI_RawMotion);

                let mut event_mask = xinput2::XIEventMask {
                    deviceid: xinput2::XIAllMasterDevices,
                    mask_len: mask.len() as i32,
                    mask: mask.as_mut_ptr(),
                };

                if xinput2::XISelectEvents(display, root_window, &mut event_mask, 1)
                    != xlib::Success as i32
                {
                    xlib::XCloseDisplay(display);
                    INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
                    log::warn!("桌宠输入联动注册失败：XInput2 事件订阅失败");
                    return;
                }

                xlib::XFlush(display);

                loop {
                    let mut event: xlib::XEvent = mem::zeroed();
                    xlib::XNextEvent(display, &mut event);

                    if event.get_type() != xlib::GenericEvent {
                        continue;
                    }

                    let cookie = &mut event.generic_event_cookie;
                    if cookie.extension != opcode
                        || xlib::XGetEventData(display, cookie) != xlib::True
                    {
                        continue;
                    }

                    match cookie.evtype {
                        xinput2::XI_RawKeyPress => {
                            let raw_event = &*(cookie.data as *const xinput2::XIRawEvent);
                            let keysym =
                                xlib::XkbKeycodeToKeysym(display, raw_event.detail as u8, 0, 0);
                            if let Some(key_code) = linux_keysym_to_avatar_key_code(keysym as u64) {
                                record_keyboard_input(
                                    standard_keyboard_group_from_key_code(key_code),
                                    key_code,
                                );
                            }
                        }
                        xinput2::XI_RawButtonPress => {
                            let raw_event = &*(cookie.data as *const xinput2::XIRawEvent);
                            record_mouse_input(linux_mouse_group_from_button_detail(
                                raw_event.detail,
                            ));
                            if let Some((cursor_ratio_x, cursor_ratio_y)) =
                                linux_query_cursor_ratio(display, root_window, screen)
                            {
                                record_cursor_ratio(cursor_ratio_x, cursor_ratio_y);
                            }
                        }
                        xinput2::XI_RawMotion => {
                            record_mouse_input(MOUSE_GROUP_MOVE);
                            if let Some((cursor_ratio_x, cursor_ratio_y)) =
                                linux_query_cursor_ratio(display, root_window, screen)
                            {
                                record_cursor_ratio(cursor_ratio_x, cursor_ratio_y);
                            }
                        }
                        _ => {}
                    }

                    xlib::XFreeEventData(display, cookie);
                }
            });
        }
        LinuxDesktopSession::Wayland => {
            let Some(provider) = linux_wayland_mouse_provider_for_environment(
                desktop_environment,
                gnome_mouse_provider_available,
                kde_mouse_provider_available,
                hyprland_mouse_provider_available,
            ) else {
                INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
                log::warn!("桌宠输入联动注册失败：未找到可用的 Wayland 鼠标 provider");
                return;
            };

            let app = app.clone();
            thread::spawn(move || {
                let mut last_mouse_state: Option<(i32, i32, u8)> = None;
                let mut last_keyboard_timestamp_ms: Option<u64> = None;

                loop {
                    if let Some(snapshot) = query_wayland_mouse_cursor_point(provider) {
                        let next_mouse_state = (snapshot.x, snapshot.y, snapshot.group_code);
                        if last_mouse_state != Some(next_mouse_state) {
                            record_mouse_input(snapshot.group_code);
                            last_mouse_state = Some(next_mouse_state);
                        }

                        if snapshot.keyboard_timestamp_ms != last_keyboard_timestamp_ms {
                            if let Some(timestamp_ms) = snapshot.keyboard_timestamp_ms {
                                if let Some(key_code) = snapshot.keyboard_key_code {
                                    record_keyboard_input(
                                        standard_keyboard_group_from_key_code(key_code),
                                        key_code,
                                    );
                                }
                                last_keyboard_timestamp_ms = Some(timestamp_ms);
                            }
                        }

                        if let Some((min_x, min_y, width, height)) =
                            virtual_desktop_bounds_from_monitors(&app)
                        {
                            let (cursor_ratio_x, cursor_ratio_y) =
                                cursor_ratio_from_virtual_desktop_bounds(
                                    snapshot.x, snapshot.y, min_x, min_y, width, height,
                                );
                            record_cursor_ratio(cursor_ratio_x, cursor_ratio_y);
                        }
                    }

                    thread::sleep(WAYLAND_MOUSE_POLL_INTERVAL);
                }
            });
        }
        LinuxDesktopSession::Unknown => {
            INPUT_MONITOR_STARTED.store(false, Ordering::SeqCst);
            log::warn!("桌宠输入联动注册失败：Linux 会话类型未知");
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn start_avatar_input_monitor(_app: &AppHandle) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linux_session::{LinuxDesktopEnvironment, LinuxDesktopSession};

    #[test]
    fn 输入活跃窗口应只在短时间内保持有效() {
        assert!(!is_input_still_active(0, 1000, 180));
        assert!(is_input_still_active(900, 1000, 180));
        assert!(!is_input_still_active(700, 1000, 180));
    }

    #[test]
    fn 输入载荷应根据最近输入时间生成键鼠活跃状态() {
        LAST_KEYBOARD_INPUT_AT_MS.store(1000, Ordering::Relaxed);
        LAST_MOUSE_INPUT_AT_MS.store(850, Ordering::Relaxed);
        LAST_KEYBOARD_GROUP_CODE.store(KEYBOARD_GROUP_KEY_Q, Ordering::Relaxed);
        LAST_KEYBOARD_KEY_CODE.store(12, Ordering::Relaxed);
        LAST_MOUSE_GROUP_CODE.store(MOUSE_GROUP_RIGHT, Ordering::Relaxed);
        CURSOR_RATIO_X_PERMILLE.store(250, Ordering::Relaxed);
        CURSOR_RATIO_Y_PERMILLE.store(750, Ordering::Relaxed);

        let payload = build_avatar_input_payload(1030);
        assert!(payload.keyboard_active);
        assert!(payload.mouse_active);
        assert_eq!(payload.keyboard_group, "key-q");
        assert_eq!(payload.keyboard_visual_key, "KeyQ");
        assert_eq!(payload.mouse_group, "mouse-right");
        assert_eq!(payload.cursor_ratio_x, 0.25);
        assert_eq!(payload.cursor_ratio_y, 0.75);

        let payload = build_avatar_input_payload(1200);
        assert!(!payload.keyboard_active);
        assert!(!payload.mouse_active);
    }

    #[test]
    fn 键盘键码应映射到原版键区分组() {
        assert_eq!(
            standard_keyboard_group_from_key_code(18),
            KEYBOARD_GROUP_DIGIT_1
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(26),
            KEYBOARD_GROUP_DIGIT_7
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(12),
            KEYBOARD_GROUP_KEY_Q
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(14),
            KEYBOARD_GROUP_KEY_E
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(15),
            KEYBOARD_GROUP_KEY_R
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(49),
            KEYBOARD_GROUP_SPACE
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(0),
            KEYBOARD_GROUP_KEY_A
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(2),
            KEYBOARD_GROUP_KEY_D
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(1),
            KEYBOARD_GROUP_KEY_S
        );
        assert_eq!(
            standard_keyboard_group_from_key_code(13),
            KEYBOARD_GROUP_KEY_W
        );
        assert_eq!(standard_keyboard_group_from_key_code(123), 0);
    }

    #[test]
    fn 键盘键码应映射到源资源图层名称() {
        assert_eq!(keyboard_visual_key_from_key_code(0), "KeyA");
        assert_eq!(keyboard_visual_key_from_key_code(45), "KeyN");
        assert_eq!(keyboard_visual_key_from_key_code(31), "KeyO");
        assert_eq!(keyboard_visual_key_from_key_code(35), "KeyP");
        assert_eq!(keyboard_visual_key_from_key_code(25), "Num9");
        assert_eq!(keyboard_visual_key_from_key_code(49), "Space");
        assert_eq!(keyboard_visual_key_from_key_code(36), "Return");
        assert_eq!(keyboard_visual_key_from_key_code(56), "ShiftLeft");
        assert_eq!(keyboard_visual_key_from_key_code(62), "ControlRight");
        assert_eq!(keyboard_visual_key_from_key_code(43), "Comma");
        assert_eq!(keyboard_visual_key_from_key_code(47), "Period");
        assert_eq!(keyboard_visual_key_from_key_code(123), "LeftArrow");
        assert_eq!(keyboard_visual_key_from_key_code(124), "RightArrow");
        assert_eq!(keyboard_visual_key_from_key_code(125), "DownArrow");
        assert_eq!(keyboard_visual_key_from_key_code(126), "UpArrow");
        assert_eq!(keyboard_visual_key_from_key_code(127), "");
    }

    #[test]
    fn 鼠标分组标签应映射到原版鼠标模式名称() {
        assert_eq!(mouse_group_label(MOUSE_GROUP_MOVE), "mouse-move");
        assert_eq!(mouse_group_label(MOUSE_GROUP_LEFT), "mouse-left");
        assert_eq!(mouse_group_label(MOUSE_GROUP_RIGHT), "mouse-right");
        assert_eq!(mouse_group_label(MOUSE_GROUP_SIDE), "mouse-side");
    }

    #[test]
    fn linux_x11会话应支持桌宠输入联动() {
        use crate::linux_session::LinuxDesktopSession;

        assert!(linux_session_supports_avatar_input(
            LinuxDesktopSession::X11
        ));
        assert!(!linux_session_supports_avatar_input(
            LinuxDesktopSession::Wayland
        ));
        assert!(!linux_session_supports_avatar_input(
            LinuxDesktopSession::Unknown
        ));
    }

    #[test]
    fn linux_keysym应映射到桌宠可消费的统一键码() {
        assert_eq!(linux_keysym_to_avatar_key_code(0x0061), Some(0));
        assert_eq!(linux_keysym_to_avatar_key_code(0x0077), Some(13));
        assert_eq!(linux_keysym_to_avatar_key_code(0x0031), Some(18));
        assert_eq!(linux_keysym_to_avatar_key_code(0xFF08), Some(51));
        assert_eq!(linux_keysym_to_avatar_key_code(0xFF53), Some(124));
        assert_eq!(linux_keysym_to_avatar_key_code(0xFFEB), Some(55));
        assert_eq!(linux_keysym_to_avatar_key_code(0xFFBE), None);
    }

    #[test]
    fn linux鼠标按钮应映射到桌宠鼠标分组() {
        assert_eq!(linux_mouse_group_from_button_detail(1), MOUSE_GROUP_LEFT);
        assert_eq!(linux_mouse_group_from_button_detail(3), MOUSE_GROUP_RIGHT);
        assert_eq!(linux_mouse_group_from_button_detail(8), MOUSE_GROUP_SIDE);
        assert_eq!(linux_mouse_group_from_button_detail(99), MOUSE_GROUP_MOVE);
    }

    #[test]
    fn kde_wayland应识别为仅鼠标联动() {
        let support = linux_avatar_input_support_for_session(
            LinuxDesktopSession::Wayland,
            LinuxDesktopEnvironment::Kde,
            false,
            true,
            false,
        );

        assert_eq!(support.support_level, "mouse-only");
        assert_eq!(support.provider, "kdotool-mouselocation");
        assert!(!support.keyboard_supported);
        assert!(support.mouse_supported);
    }

    #[test]
    fn hyprland_wayland应识别为仅鼠标联动() {
        let support = linux_avatar_input_support_for_session(
            LinuxDesktopSession::Wayland,
            LinuxDesktopEnvironment::Hyprland,
            false,
            false,
            true,
        );

        assert_eq!(support.support_level, "mouse-only");
        assert_eq!(support.provider, "hyprctl-cursorpos");
        assert!(!support.keyboard_supported);
        assert!(support.mouse_supported);
    }

    #[test]
    fn gnome_wayland在缺少provider时应视为不可用() {
        let support = linux_avatar_input_support_for_session(
            LinuxDesktopSession::Wayland,
            LinuxDesktopEnvironment::Gnome,
            false,
            false,
            false,
        );

        assert_eq!(support.support_level, "none");
        assert_eq!(support.provider, "none");
        assert!(!support.keyboard_supported);
        assert!(!support.mouse_supported);
    }

    #[test]
    fn kdotool鼠标位置输出应解析为坐标() {
        assert_eq!(
            parse_kdotool_mouse_location_output("x:1489 y:812 screen:0 window:50331653"),
            Some((1489, 812))
        );
        assert_eq!(parse_kdotool_mouse_location_output("x:abc y:812"), None);
    }

    #[test]
    fn gnome_wayland在扩展可用时应识别为完整联动() {
        let support = linux_avatar_input_support_for_session(
            LinuxDesktopSession::Wayland,
            LinuxDesktopEnvironment::Gnome,
            true,
            false,
            false,
        );

        assert_eq!(support.support_level, "full");
        assert_eq!(support.provider, "gnome-shell-dbus");
        assert!(support.keyboard_supported);
        assert!(support.mouse_supported);
    }

    #[test]
    fn gnome_avatar_input_dbus输出应解析坐标鼠标分组与最近按键() {
        let output = "('{\"x\":1489,\"y\":812,\"mouseGroup\":\"mouse-right\",\"keyval\":113,\"keycode\":24,\"keyboardTimestampMs\":123456}','')";
        assert_eq!(
            parse_gnome_avatar_input_dbus_output(output),
            Some(LinuxWaylandMouseSnapshot {
                x: 1489,
                y: 812,
                group_code: MOUSE_GROUP_RIGHT,
                keyboard_key_code: Some(12),
                keyboard_timestamp_ms: Some(123456),
            })
        );
    }

    #[test]
    fn hyprctl_cursorpos输出应解析为坐标() {
        assert_eq!(
            parse_hyprctl_cursorpos_output("1489, 812"),
            Some((1489, 812))
        );
        assert_eq!(
            parse_hyprctl_cursorpos_output("1489 812"),
            Some((1489, 812))
        );
        assert_eq!(parse_hyprctl_cursorpos_output("oops"), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows虚拟键应映射到桌宠可消费的统一键码() {
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x41), Some(0));
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x53), Some(1));
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x44), Some(2));
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x57), Some(13));
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x20), Some(49));
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x25), Some(123));
        assert_eq!(windows_virtual_key_to_avatar_key_code(0x70), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows鼠标消息应映射到桌宠鼠标分组() {
        use winapi::um::winuser::{
            WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
        };

        assert_eq!(
            windows_mouse_group_from_message(WM_MOUSEMOVE),
            MOUSE_GROUP_MOVE
        );
        assert_eq!(
            windows_mouse_group_from_message(WM_LBUTTONDOWN),
            MOUSE_GROUP_LEFT
        );
        assert_eq!(
            windows_mouse_group_from_message(WM_RBUTTONDOWN),
            MOUSE_GROUP_RIGHT
        );
        assert_eq!(
            windows_mouse_group_from_message(WM_MBUTTONDOWN),
            MOUSE_GROUP_SIDE
        );
        assert_eq!(
            windows_mouse_group_from_message(WM_XBUTTONDOWN),
            MOUSE_GROUP_SIDE
        );
    }
}
