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
    /// 使用多种方法检测，避免依赖 Python/pyobjc
    #[cfg(target_os = "macos")]
    pub fn is_locked(&self) -> bool {
        use std::process::Command;

        // 方法0 (最可靠): CGSessionCopyCurrentDictionary
        // 返回当前登录会话字典，包含 CGSSessionScreenIsLocked 键
        // 锁屏、睡眠、Power Nap 唤醒等场景均能准确检测
        if Self::is_session_locked() {
            log::debug!("锁屏检测: CGSession 报告屏幕已锁定");
            return true;
        }

        // 方法1: 检查是否有屏幕保护程序运行
        let output = Command::new("pgrep")
            .args(["-x", "ScreenSaverEngine"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                log::debug!("锁屏检测: 屏幕保护程序运行中");
                return true;
            }
        }

        // 方法2: 使用 ioreg 检测显示器电源状态
        // 当屏幕关闭（锁屏后自动关闭）时，IODisplayWrangler 的 DevicePowerState 为 0
        let output = Command::new("ioreg")
            .args(["-r", "-c", "IODisplayWrangler", "-d", "1"])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // 检查 DevicePowerState = 0 表示显示器已关闭
            if stdout.contains("\"DevicePowerState\" = 0") {
                log::debug!("锁屏检测: 显示器已关闭");
                return true;
            }
        }

        // 方法3: 使用 osascript 检查屏幕保护状态
        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to return running of screen saver preferences",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim() == "true" {
                log::debug!("锁屏检测: 屏幕保护已激活");
                return true;
            }
        }

        // 方法4: 检查 loginwindow 进程是否在前台（用户在锁屏界面）
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first application process whose frontmost is true"])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let frontmost = stdout.trim().to_lowercase();
            if frontmost == "loginwindow" || frontmost == "screensaverengine" {
                log::debug!("锁屏检测: 前台应用为锁屏界面");
                return true;
            }
        }

        false
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

    /// 检查屏幕是否锁定 (其他平台)
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
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
        let current = (now.hour() as u8, now.minute() as u8);
        let start = (start_hour, start_minute);
        let end = (end_hour, end_minute);

        if start <= end {
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
