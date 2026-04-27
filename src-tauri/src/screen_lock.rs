// 锁屏检测模块 (Windows / macOS)
// 监听系统锁屏/解锁事件，用于控制录制状态

#![allow(dead_code)]

use chrono::Timelike;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 屏幕锁定状态
pub struct ScreenLockMonitor {
    /// 是否锁定
    is_locked: Arc<AtomicBool>,
}

impl ScreenLockMonitor {
    /// 创建锁屏监控器
    pub fn new() -> Self {
        Self {
            is_locked: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 检查屏幕是否锁定 (Windows)
    /// 使用 OpenInputDesktop 方式判断：锁屏时系统桌面切换到 Winlogon 桌面，
    /// 此时当前线程无法打开输入桌面，可靠性远高于 GetForegroundWindow/quser
    #[cfg(target_os = "windows")]
    pub fn is_locked(&self) -> bool {
        use winapi::um::winnt::GENERIC_ALL;
        use winapi::um::winuser::{CloseDesktop, OpenInputDesktop, SwitchDesktop};

        unsafe {
            // 尝试打开当前输入桌面
            // 锁屏时系统会切换到 Winlogon 桌面，当前进程无权限打开，返回 null
            let desktop = OpenInputDesktop(0, 0, GENERIC_ALL);
            if desktop.is_null() {
                // 无法打开输入桌面，说明已经锁屏
                log::debug!("锁屏检测: OpenInputDesktop 返回 null，判断为锁屏");
                return true;
            }

            // 尝试切换到该桌面（如果切换失败，说明是受限的 Winlogon 桌面）
            let switched = SwitchDesktop(desktop);
            CloseDesktop(desktop);

            if switched == 0 {
                // SwitchDesktop 失败，说明是锁屏桌面
                log::debug!("锁屏检测: SwitchDesktop 失败，判断为锁屏");
                return true;
            }
        }

        false
    }

    /// 检查屏幕是否锁定 (macOS)
    /// 仅使用 CGSessionCopyCurrentDictionary (FFI)，不再 spawn 外部进程
    /// CGSession 覆盖锁屏、睡眠、Power Nap 唤醒等全部场景，是最可靠的检测方式
    #[cfg(target_os = "macos")]
    pub fn is_locked(&self) -> bool {
        // CGSessionCopyCurrentDictionary: 纯 FFI 调用，无进程 spawn 开销
        // 返回当前登录会话字典，包含 CGSSessionScreenIsLocked 键
        let locked = Self::is_session_locked();
        if locked {
            log::debug!("锁屏检测: CGSession 报告屏幕已锁定");
        }
        locked
    }

    /// macOS: 通过 CGSessionCopyCurrentDictionary 检测锁屏
    /// 这是最可靠的方式，在 Power Nap 唤醒、合盖睡眠等场景均可准确检测
    #[cfg(target_os = "macos")]
    fn is_session_locked() -> bool {
        use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionaryRef;
        use core_foundation::string::CFString;

        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
        }

        unsafe {
            let dict = CGSessionCopyCurrentDictionary();
            if dict.is_null() {
                // 无法获取会话信息（可能在睡眠或无用户登录），视为锁定
                return true;
            }

            let key = CFString::new("CGSSessionScreenIsLocked");
            let mut value_ref: CFTypeRef = std::ptr::null();
            let found = core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict,
                key.as_CFTypeRef() as *const _,
                &mut value_ref,
            );

            let locked = if found != 0 && !value_ref.is_null() {
                // 值是 CFBoolean，检查是否为 true
                let cf_bool = CFBoolean::wrap_under_get_rule(value_ref as _);
                cf_bool == CFBoolean::true_value()
            } else {
                false
            };

            CFRelease(dict as _);
            locked
        }
    }

    /// 检查屏幕是否锁定 (Linux)
    /// 通过 D-Bus 查询 screensaver 状态或检查锁屏进程
    #[cfg(target_os = "linux")]
    pub fn is_locked(&self) -> bool {
        use std::process::Command;

        // 方法1: 通过 loginctl 检查 session 是否 locked
        if let Ok(output) = Command::new("loginctl")
            .args([
                "show-session",
                "auto",
                "--property=LockedHint",
                "--no-legend",
            ])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("LockedHint=yes") {
                log::debug!("锁屏检测: loginctl 报告 session 已锁定");
                return true;
            }
        }

        // 方法2: 检查常见锁屏进程
        for proc_name in &[
            "cinnamon-screensaver",
            "gnome-screensaver",
            "xscreensaver",
            "i3lock",
            "swaylock",
        ] {
            if let Ok(output) = Command::new("pgrep").args(["-x", proc_name]).output() {
                if output.status.success() {
                    log::debug!("锁屏检测: 锁屏进程 {} 运行中", proc_name);
                    return true;
                }
            }
        }

        // 方法3: D-Bus 查询 Cinnamon/GNOME screensaver
        if let Ok(output) = Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.cinnamon.ScreenSaver",
                "--type=method_call",
                "--print-reply",
                "/org/cinnamon/ScreenSaver",
                "org.cinnamon.ScreenSaver.GetActive",
            ])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("boolean true") {
                log::debug!("锁屏检测: Cinnamon ScreenSaver 报告已激活");
                return true;
            }
        }

        false
    }

    /// 检查屏幕是否锁定 (其他平台)
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn is_locked(&self) -> bool {
        false
    }

    /// 设置锁定状态（用于手动更新）
    pub fn set_locked(&self, locked: bool) {
        self.is_locked.store(locked, Ordering::SeqCst);
    }

    /// 检查是否在工作时间内
    pub fn is_work_time(start_hour: u8, start_minute: u8, end_hour: u8, end_minute: u8) -> bool {
        let now = chrono::Local::now();
        Self::is_work_time_range(
            (now.hour() as u8, now.minute() as u8),
            (start_hour.min(23), start_minute.min(59)),
            (end_hour.min(23), end_minute.min(59)),
        )
    }

    /// 检查是否在任一工作时间段内
    pub fn is_work_time_in_segments(segments: &[crate::config::WorkTimeSegment]) -> bool {
        let now = chrono::Local::now();
        Self::is_work_time_in_segments_at((now.hour() as u8, now.minute() as u8), segments)
    }

    fn is_work_time_in_segments_at(
        current: (u8, u8),
        segments: &[crate::config::WorkTimeSegment],
    ) -> bool {
        segments.iter().any(|segment| {
            let start = (segment.start_hour.min(23), segment.start_minute.min(59));
            let end = (segment.end_hour.min(23), segment.end_minute.min(59));
            Self::is_work_time_range(current, start, end)
        })
    }

    fn is_work_time_range(current: (u8, u8), start: (u8, u8), end: (u8, u8)) -> bool {
        if start == end {
            return false;
        }

        if start < end {
            // 正常时间范围，如 9:00-18:00 或 8:30-17:30
            current >= start && current < end
        } else {
            // 跨午夜，如 22:00-6:00
            current >= start || current < end
        }
    }
}

impl Default for ScreenLockMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ScreenLockMonitor;
    use crate::config::WorkTimeSegment;

    #[test]
    fn 开始时间等于结束时间时不应视为工作时间() {
        assert!(!ScreenLockMonitor::is_work_time(9, 0, 9, 0));
        assert!(!ScreenLockMonitor::is_work_time(0, 0, 0, 0));
    }

    #[test]
    fn 多段工作时间应在任一时段内返回真() {
        let segments = vec![
            WorkTimeSegment {
                start_hour: 9,
                start_minute: 0,
                end_hour: 12,
                end_minute: 0,
            },
            WorkTimeSegment {
                start_hour: 13,
                start_minute: 0,
                end_hour: 18,
                end_minute: 0,
            },
        ];

        assert!(ScreenLockMonitor::is_work_time_in_segments_at(
            (10, 30),
            &segments
        ));
        assert!(ScreenLockMonitor::is_work_time_in_segments_at(
            (13, 10),
            &segments
        ));
        assert!(!ScreenLockMonitor::is_work_time_in_segments_at(
            (12, 30),
            &segments
        ));
    }
}
