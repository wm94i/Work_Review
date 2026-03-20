use crate::error::{AppError, Result};

/// 活动窗口信息
#[derive(Debug, Clone)]
pub struct ActiveWindow {
    pub app_name: String,
    pub window_title: String,
    /// 浏览器 URL（如果当前应用是浏览器）
    pub browser_url: Option<String>,
}

/// 判断进程名是否属于浏览器
pub fn is_browser_app(app_name: &str) -> bool {
    let app_lower = app_name.to_lowercase();
    app_lower.contains("chrome")
        || app_lower.contains("msedge")
        || app_lower.contains("brave")
        || app_lower.contains("opera")
        || app_lower.contains("vivaldi")
        || app_lower.contains("firefox")
        || app_lower.contains("safari")
        || app_lower.contains("arc")
        || app_lower.contains("orion")
        || app_lower.contains("zen browser")
        || app_lower.contains("browser")
        || app_lower.contains("360se")
        || app_lower.contains("360chrome")
        || app_lower.contains("qqbrowser")
        || app_lower.contains("sogouexplorer")
        || app_lower.contains("2345explorer")
        || app_lower.contains("liebao")
        || app_lower.contains("maxthon")
        || app_lower.contains("theworld")
        || app_lower.contains("cent")
        || app_lower.contains("iexplore")
}

/// 统一应用显示名称，避免不同来源（进程名、数据库历史、运行中列表）出现重复项
pub fn normalize_display_app_name(app_name: &str) -> String {
    let trimmed = app_name
        .trim()
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .trim();

    let normalized = trimmed.to_lowercase();
    match normalized.as_str() {
        "work-review" | "work_review" | "workreview" | "work review" => {
            "Work Review".to_string()
        }
        "chrome" | "google chrome" => "Google Chrome".to_string(),
        "msedge" | "edge" | "microsoft edge" => "Microsoft Edge".to_string(),
        "brave" | "brave browser" => "Brave Browser".to_string(),
        "firefox" => "Firefox".to_string(),
        "safari" => "Safari".to_string(),
        "opera" => "Opera".to_string(),
        "vivaldi" => "Vivaldi".to_string(),
        "chromium" => "Chromium".to_string(),
        "arc" => "Arc".to_string(),
        "zen browser" | "zen" => "Zen Browser".to_string(),
        "qqbrowser" | "qq浏览器" => "QQ Browser".to_string(),
        "360se" | "360chrome" | "360浏览器" => "360 Browser".to_string(),
        "sogouexplorer" | "搜狗浏览器" => "Sogou Browser".to_string(),
        "code" | "vscode" | "visual studio code" | "vs code" => "VS Code".to_string(),
        "cursor" => "Cursor".to_string(),
        "wechat" | "weixin" | "微信" => "WeChat".to_string(),
        "wecom" | "企业微信" => "WeCom".to_string(),
        "qq" => "QQ".to_string(),
        "notion" => "Notion".to_string(),
        "obsidian" => "Obsidian".to_string(),
        "slack" => "Slack".to_string(),
        "discord" => "Discord".to_string(),
        "winword" | "word" => "Microsoft Word".to_string(),
        "excel" => "Microsoft Excel".to_string(),
        "powerpnt" | "powerpoint" => "Microsoft PowerPoint".to_string(),
        "outlook" => "Microsoft Outlook".to_string(),
        "explorer" => "File Explorer".to_string(),
        "windowsterminal" | "windows terminal" => "Windows Terminal".to_string(),
        "powershell" | "pwsh" => "PowerShell".to_string(),
        "cmd" => "Command Prompt".to_string(),
        _ => trimmed.to_string(),
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn is_probable_domain(value: &str) -> bool {
    let candidate = value.trim().trim_matches('/').to_lowercase();
    if candidate.is_empty()
        || candidate.contains(' ')
        || candidate.starts_with('.')
        || candidate.ends_with('.')
        || !candidate.contains('.')
    {
        return false;
    }

    let labels: Vec<&str> = candidate.split('.').collect();
    if labels.len() < 2 {
        return false;
    }

    let tld = labels.last().copied().unwrap_or_default();
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }

    labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn normalize_possible_url(value: &str) -> Option<String> {
    let candidate = value
        .trim()
        .trim_matches(|c: char| c.is_control() || c == '\u{200b}' || c == '\u{feff}');

    if candidate.is_empty() {
        return None;
    }

    if candidate.contains(' ') {
        return None;
    }

    let candidate_lower = candidate.to_lowercase();
    if candidate_lower.starts_with("http://") || candidate_lower.starts_with("https://") {
        return Some(candidate.to_string());
    }

    if candidate.contains("://")
        || candidate_lower.starts_with("about:")
        || candidate_lower.starts_with("chrome:")
        || candidate_lower.starts_with("edge:")
        || candidate_lower.starts_with("file:")
    {
        return Some(candidate.to_string());
    }

    if is_probable_domain(candidate) {
        return Some(format!("https://{}", candidate.trim_end_matches('/')));
    }

    None
}

/// 获取当前活动窗口信息
#[cfg(target_os = "windows")]
pub fn get_active_window() -> Result<ActiveWindow> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::psapi::GetModuleBaseNameW;
    use winapi::um::winnt::PROCESS_QUERY_INFORMATION;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    // PROCESS_QUERY_LIMITED_INFORMATION 是 Vista+ 专为低权限场景设计的标志
    // 无需 PROCESS_VM_READ，对 UAC 保护进程、Store 应用等成功率远高于完整权限
    const PROCESS_QUERY_LIMITED: u32 = 0x1000;

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            // Win10 上切换窗口/UAC弹窗时可能返回 null，不应直接报错
            // 降级返回 Desktop，确保不丢失轮询周期的时长
            return Ok(ActiveWindow {
                app_name: "Desktop".to_string(),
                window_title: String::new(),
                browser_url: None,
            });
        }

        // 获取窗口标题
        let mut title: [u16; 512] = [0; 512];
        let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 512);
        let window_title = if len > 0 {
            OsString::from_wide(&title[..len as usize])
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        // 获取进程ID
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);

        // 获取进程名，使用多级备用策略确保 Win10 低权限下能正确读取
        let app_name = if pid > 0 {
            // 方法一：PROCESS_QUERY_LIMITED_INFORMATION + GetModuleBaseNameW
            // 对大多数普通进程（Word、VSCode、WPS 等）有效
            let handle = OpenProcess(PROCESS_QUERY_LIMITED, 0, pid);
            let name_opt = if !handle.is_null() {
                let mut name: [u16; 256] = [0; 256];
                let len = GetModuleBaseNameW(handle, std::ptr::null_mut(), name.as_mut_ptr(), 256);
                CloseHandle(handle);
                if len > 0 {
                    Some(
                        OsString::from_wide(&name[..len as usize])
                            .to_string_lossy()
                            .to_string(),
                    )
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(n) = name_opt {
                n
            } else {
                // 方法二：回退完整权限（覆盖 GetModuleBaseNameW 需要 PROCESS_VM_READ 的场景）
                let handle2 = OpenProcess(PROCESS_QUERY_INFORMATION | 0x0010, 0, pid);
                let name_opt2 = if !handle2.is_null() {
                    let mut name: [u16; 256] = [0; 256];
                    let len =
                        GetModuleBaseNameW(handle2, std::ptr::null_mut(), name.as_mut_ptr(), 256);
                    CloseHandle(handle2);
                    if len > 0 {
                        Some(
                            OsString::from_wide(&name[..len as usize])
                                .to_string_lossy()
                                .to_string(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(n) = name_opt2 {
                    n
                } else {
                    // 方法三：QueryFullProcessImageNameW，只需低权限，返回完整路径取文件名
                    get_process_name_by_image(pid).unwrap_or_else(|| {
                        // 方法四：从窗口标题最后一段推断（如 "文件名 - 应用名" 取最后段）
                        // 避免进程全部落入 Unknown 导致时长无法区分统计
                        if !window_title.is_empty() {
                            window_title
                                .split(" - ")
                                .last()
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty() && s.len() < 40)
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Unknown".to_string()
                        }
                    })
                }
            }
        } else {
            "Unknown".to_string()
        };

        // 尝试获取浏览器 URL (Windows)
        let browser_url = get_browser_url_windows(&app_name, &window_title, hwnd as isize);

        Ok(ActiveWindow {
            app_name,
            window_title,
            browser_url,
        })
    }
}

/// 通过 QueryFullProcessImageNameW 获取进程可执行文件名，仅需低权限
/// 返回 exe 文件名（不含路径，如 "WINWORD.EXE"），作为 GetModuleBaseNameW 的备用
#[cfg(target_os = "windows")]
fn get_process_name_by_image(pid: u32) -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winbase::QueryFullProcessImageNameW;

    unsafe {
        // 只需 PROCESS_QUERY_LIMITED_INFORMATION，对 UAC 保护进程也有效
        let handle = OpenProcess(0x1000, 0, pid);
        if handle.is_null() {
            return None;
        }

        let mut buf: [u16; 512] = [0; 512];
        let mut size: u32 = 512;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);

        if ok == 0 || size == 0 {
            return None;
        }

        // 返回完整路径（如 C:\...\WINWORD.EXE），提取最后一段作为进程名
        let full_path = OsString::from_wide(&buf[..size as usize])
            .to_string_lossy()
            .to_string();

        full_path
            .split('\\')
            .last()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }
}

/// 从窗口获取浏览器 URL (Windows)
/// 使用原生 UI Automation COM 接口（通过 uiautomation crate），不再 spawn PowerShell 进程
/// 为避免串号，不缓存正向结果，优先保证 URL 与时长归属的准确性
#[cfg(target_os = "windows")]
fn get_browser_url_windows(app_name: &str, window_title: &str, hwnd: isize) -> Option<String> {
    if !is_browser_app(app_name) {
        return None;
    }

    // 使用原生 UI Automation 获取 URL，catch_unwind 防止 COM 异常导致崩溃
    let result = std::panic::catch_unwind(|| get_url_via_uiautomation(hwnd)).unwrap_or(None);

    // UI Automation 失败时，尝试从窗口标题提取域名信息作为兜底
    result.or_else(|| extract_url_from_title(window_title))
}

/// 通过原生 UI Automation COM 接口获取浏览器地址栏 URL
/// 使用 HWND 精准定位浏览器窗口，查找 Edit 控件并读取 ValuePattern
#[cfg(target_os = "windows")]
fn get_url_via_uiautomation(hwnd: isize) -> Option<String> {
    use uiautomation::patterns::{UILegacyIAccessiblePattern, UIValuePattern};
    use uiautomation::types::{ControlType, Handle};
    use uiautomation::UIAutomation;

    let automation = UIAutomation::new().ok()?;
    // Handle 内部字段在 0.24.4 变为私有，改用 From trait 构造
    let window_element = automation.element_from_handle(Handle::from(hwnd)).ok()?;

    let mut best_match: Option<(i32, String)> = None;

    let inspect_control = |control: uiautomation::UIElement,
                           best_match: &mut Option<(i32, String)>| {
        let control_type = match control.get_control_type() {
            Ok(t) => t,
            Err(_) => return,
        };

        if control_type != ControlType::Edit && control_type != ControlType::Document {
            return;
        }

        let name = control.get_name().unwrap_or_default();
        let class_name = control.get_classname().unwrap_or_default();
        let name_lower = name.to_lowercase();
        let class_lower = class_name.to_lowercase();
        let address_like = name_lower.contains("address")
            || name_lower.contains("地址")
            || name_lower.contains("location")
            || name_lower.contains("omnibox")
            || class_lower.contains("omnibox")
            || class_lower.contains("address");

        let mut candidates = Vec::new();
        if let Ok(pattern) = control.get_pattern::<UIValuePattern>() {
            if let Ok(value) = pattern.get_value() {
                candidates.push(value);
            }
        }
        if let Ok(pattern) = control.get_pattern::<UILegacyIAccessiblePattern>() {
            if let Ok(value) = pattern.get_value() {
                candidates.push(value);
            }
        }
        candidates.push(name.clone());

        for raw in candidates {
            let Some(url) = normalize_possible_url(&raw) else {
                continue;
            };

            let mut score = match control_type {
                ControlType::Edit => 35,
                ControlType::Document => 15,
                _ => 0,
            };

            if address_like {
                score += 50;
            }
            if raw.starts_with("http://") || raw.starts_with("https://") {
                score += 30;
            } else if raw == class_name || raw == name {
                score += 5;
            }

            if score >= 60
                && best_match
                    .as_ref()
                    .map(|(best_score, _)| score > *best_score)
                    .unwrap_or(true)
            {
                *best_match = Some((score, url));
            }
        }
    };

    // 先扫描全部 Edit 控件。
    // Chrome/Chromium 的地址栏在不同版本和 UI 状态下不一定是第一个 Edit；
    // 只取 find_first 很容易误拿到页面内搜索框，导致 URL 统计长期为空。
    if let Ok(edits) = automation
        .create_matcher()
        .from(window_element.clone())
        .control_type(ControlType::Edit)
        .timeout(300)
        .find_all()
    {
        for edit in edits {
            inspect_control(edit, &mut best_match);
        }
    }
    if let Some((score, url)) = &best_match {
        if *score >= 85 {
            return Some(url.clone());
        }
    }

    // 再扫 Document 控件作为补充。
    // 某些浏览器或特殊页面会把可读 URL 暴露在 Document，而不是地址栏 Edit。
    if let Ok(docs) = automation
        .create_matcher()
        .from(window_element)
        .control_type(ControlType::Document)
        .timeout(300)
        .find_all()
    {
        for doc in docs {
            inspect_control(doc, &mut best_match);
        }
    }

    best_match.map(|(_, url)| url)
}

/// 从窗口标题尝试提取 URL 或域名（UI Automation 失败时的兜底方案）
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn extract_url_from_title(window_title: &str) -> Option<String> {
    let title = window_title.trim();
    if title.is_empty() {
        return None;
    }

    // 标题本身就是 URL
    if let Some(url) = title
        .split_whitespace()
        .next()
        .and_then(normalize_possible_url)
    {
        return Some(url);
    }

    // 尝试从 "Page Title - domain.com - Browser" 格式中提取域名
    for part in title.rsplit(" - ") {
        if let Some(url) = normalize_possible_url(part) {
            return Some(url);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        extract_url_from_title, is_browser_app, is_probable_domain, normalize_possible_url,
    };

    #[test]
    fn 识别浏览器进程名() {
        assert!(is_browser_app("chrome.exe"));
        assert!(is_browser_app("msedge.exe"));
        assert!(is_browser_app("Safari"));
        assert!(!is_browser_app("Code.exe"));
    }

    #[test]
    fn 规范化地址栏候选值() {
        assert_eq!(
            normalize_possible_url("https://example.com/path"),
            Some("https://example.com/path".to_string())
        );
        assert_eq!(
            normalize_possible_url("example.com"),
            Some("https://example.com".to_string())
        );
        assert_eq!(
            normalize_possible_url("chrome://settings"),
            Some("chrome://settings".to_string())
        );
        assert_eq!(normalize_possible_url("搜索内容"), None);
        assert_eq!(normalize_possible_url("1.2.3"), None);
    }

    #[test]
    fn 从标题提取域名时避免误判() {
        assert_eq!(
            extract_url_from_title("项目文档 - docs.example.com - Google Chrome"),
            Some("https://docs.example.com".to_string())
        );
        assert_eq!(extract_url_from_title("版本 1.2.3 - Google Chrome"), None);
        assert!(is_probable_domain("sub.example.com"));
        assert!(!is_probable_domain("1.2.3"));
    }
}

/// 获取当前活动窗口信息 (macOS)
#[cfg(target_os = "macos")]
pub fn get_active_window() -> Result<ActiveWindow> {
    use std::process::Command;

    // 使用 AppleScript 获取活动应用信息
    let script = r#"
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set appName to name of frontApp
            set windowTitle to ""
            try
                set windowTitle to name of front window of frontApp
            end try
            return appName & "|" & windowTitle
        end tell
    "#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| AppError::Screenshot(format!("执行AppleScript失败: {e}")))?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = result.splitn(2, '|').collect();

        let raw_app_name = parts.first().unwrap_or(&"Unknown").to_string();
        let window_title = parts.get(1).unwrap_or(&"").to_string();

        // 对 Electron 类应用进行名称规范化
        let app_name = normalize_electron_app_name(&raw_app_name, &window_title);

        // 如果是浏览器，尝试获取 URL
        let browser_url = get_browser_url(&app_name);

        Ok(ActiveWindow {
            app_name,
            window_title,
            browser_url,
        })
    } else {
        Err(AppError::Screenshot("获取活动窗口失败".to_string()))
    }
}

/// 规范化 Electron 应用名称
/// 对于一些基于 Electron 的应用，进程名可能是 Electron 或 xxxx Helper
/// 需要根据窗口标题或其他特征识别真实应用名
#[cfg(target_os = "macos")]
fn normalize_electron_app_name(process_name: &str, window_title: &str) -> String {
    let process_lower = process_name.to_lowercase();
    let title_lower = window_title.to_lowercase();

    let process_aliases = [
        ("work-review", "Work Review"),
        ("work_review", "Work Review"),
        ("workreview", "Work Review"),
    ];

    for (pattern, real_name) in process_aliases.iter() {
        if process_lower == *pattern {
            log::debug!("进程名归一化: {process_name} -> {real_name}");
            return real_name.to_string();
        }
    }

    // 优先检查窗口标题是否包含浏览器名称
    // 这对于 Chrome 等浏览器至关重要，因为它们可能被误识别为 Electron
    let browser_patterns = [
        ("google chrome", "Google Chrome"),
        ("chrome", "Google Chrome"),
        ("safari", "Safari"),
        ("firefox", "Firefox"),
        ("microsoft edge", "Microsoft Edge"),
        ("edge", "Microsoft Edge"),
        ("arc", "Arc"),
        ("brave", "Brave Browser"),
        ("opera", "Opera"),
        ("vivaldi", "Vivaldi"),
        ("chromium", "Chromium"),
        ("orion", "Orion"),
        ("zen browser", "Zen Browser"),
        ("sidekick", "Sidekick"),
    ];

    for (pattern, browser_name) in browser_patterns.iter() {
        if title_lower.contains(pattern) {
            log::debug!(
                "浏览器识别: {process_name} -> {browser_name} (基于窗口标题: {window_title})"
            );
            return browser_name.to_string();
        }
    }

    // 如果不是 Electron 相关进程，直接返回
    if !process_lower.contains("electron") && !process_lower.contains("helper") {
        return process_name.to_string();
    }

    // Electron 应用映射表：通过窗口标题关键词识别
    let electron_apps = [
        // 编辑器/IDE
        ("cursor", "Cursor"),
        ("visual studio code", "VS Code"),
        ("vscode", "VS Code"),
        ("code - ", "VS Code"), // VS Code 窗口标题常见格式
        // AI 工具
        ("antigravity", "Antigravity"),
        ("work review", "Work Review"),
        ("copilot", "GitHub Copilot"),
        ("claude", "Claude Desktop"),
        // 通讯工具
        ("slack", "Slack"),
        ("discord", "Discord"),
        ("teams", "Microsoft Teams"),
        ("telegram", "Telegram Desktop"),
        ("whatsapp", "WhatsApp"),
        // 笔记/知识管理
        ("notion", "Notion"),
        ("obsidian", "Obsidian"),
        ("logseq", "Logseq"),
        ("roam", "Roam Research"),
        ("craft", "Craft"),
        // 其他开发工具
        ("postman", "Postman"),
        ("insomnia", "Insomnia"),
        ("figma", "Figma"),
        ("1password", "1Password"),
        ("bitwarden", "Bitwarden"),
        // 其他常见应用
        ("spotify", "Spotify"),
        ("todoist", "Todoist"),
        ("linear", "Linear"),
        ("raycast", "Raycast"),
    ];

    // 遍历映射表查找匹配
    for (keyword, real_name) in electron_apps.iter() {
        if title_lower.contains(keyword) {
            log::debug!(
                "Electron 应用识别: {process_name} -> {real_name} (基于窗口标题: {window_title})"
            );
            return real_name.to_string();
        }
    }

    // 如果窗口标题有明确的应用名格式（如 "AppName - Document"）
    // 尝试提取第一个部分作为应用名
    if let Some(first_part) = window_title.split(" - ").last() {
        let trimmed = first_part.trim();
        if !trimmed.is_empty() && trimmed.len() < 30 && !trimmed.contains('/') {
            // 检查是否像是应用名（首字母大写或全英文）
            if trimmed
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
                || trimmed
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
            {
                log::debug!("Electron 应用推断: {process_name} -> {trimmed} (从标题提取)");
                return trimmed.to_string();
            }
        }
    }

    // 无法识别，返回原始进程名
    log::debug!("无法识别 Electron 应用: {process_name} (标题: {window_title})");
    process_name.to_string()
}

/// 获取浏览器当前 URL (macOS)
/// 使用 window 1 获取最前面窗口的活动标签页 URL
#[cfg(target_os = "macos")]
fn get_browser_url(app_name: &str) -> Option<String> {
    use std::process::Command;

    let app_lower = app_name.to_lowercase();

    // 根据不同浏览器使用不同的 AppleScript
    // 使用 front window 获取最近激活的窗口的活动标签页 URL
    let (script, browser_name) =
        if app_lower.contains("chrome") || app_lower.contains("google chrome") {
            // Chrome: 使用 front window 获取最近激活的窗口
            (
                r#"tell application "Google Chrome"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Chrome",
            )
        } else if app_lower.contains("safari") {
            (
                r#"tell application "Safari"
    if (count of windows) > 0 then
        return URL of current tab of front window
    else
        return ""
    end if
end tell"#,
                "Safari",
            )
        } else if app_lower.contains("firefox") {
            // Firefox 对 AppleScript 支持有限
            (
                r#"tell application "Firefox" to get URL of front document"#,
                "Firefox",
            )
        } else if app_lower.contains("edge") {
            (
                r#"tell application "Microsoft Edge"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Edge",
            )
        } else if app_lower.contains("arc") {
            (
                r#"tell application "Arc"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Arc",
            )
        } else if app_lower.contains("brave") {
            (
                r#"tell application "Brave Browser"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Brave",
            )
        } else if app_lower.contains("opera") {
            (
                r#"tell application "Opera"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Opera",
            )
        } else if app_lower.contains("vivaldi") {
            (
                r#"tell application "Vivaldi"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Vivaldi",
            )
        } else if app_lower.contains("chromium") {
            (
                r#"tell application "Chromium"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Chromium",
            )
        } else if app_lower.contains("orion") {
            (
                r#"tell application "Orion"
    if (count of documents) > 0 then
        return URL of front document
    else
        return ""
    end if
end tell"#,
                "Orion",
            )
        } else if app_lower.contains("zen") {
            // Zen 浏览器基于 Firefox
            (
                r#"tell application "Zen Browser"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Zen",
            )
        } else if app_lower.contains("sidekick") {
            // Sidekick 基于 Chromium
            (
                r#"tell application "Sidekick"
    if (count of windows) > 0 then
        return URL of active tab of front window
    else
        return ""
    end if
end tell"#,
                "Sidekick",
            )
        } else {
            log::debug!("未识别的浏览器: {app_name}");
            return None;
        };

    log::debug!("尝试获取 {browser_name} URL: {app_name}");

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() && (url.starts_with("http") || url.starts_with("file")) {
            log::info!("获取到 {} URL: {}", browser_name, &url[..url.len().min(50)]);
            Some(url)
        } else {
            log::debug!("{browser_name} 返回空 URL");
            None
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::warn!("获取 {} URL 失败: {}", browser_name, stderr.trim());
        None
    }
}

/// 获取当前活动窗口信息 (Linux 或其他平台的后备实现)
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn get_active_window() -> Result<ActiveWindow> {
    Ok(ActiveWindow {
        app_name: "Unknown".to_string(),
        window_title: "Unknown".to_string(),
        browser_url: None,
    })
}

/// 获取浮动/overlay 窗口（如 PiP 画中画小窗）
/// 通过 CGWindowListCopyWindowInfo 枚举屏幕上所有窗口，
/// 过滤出 layer > 0 的浮动窗口（排除当前前台应用和系统进程）
#[cfg(target_os = "macos")]
pub fn get_overlay_windows(frontmost_app: &str) -> Vec<ActiveWindow> {
    use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::dictionary::CFDictionaryRef;
    use core_foundation::number::CFNumberRef;
    use core_foundation::string::CFString;
    use core_graphics::display::{
        kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
        CGWindowListCopyWindowInfo,
    };

    // 系统进程排除列表（覆盖英文和中文 macOS 系统下的进程名）
    const SYSTEM_PROCESSES: &[&str] = &[
        "Window Server",
        "Dock",
        "程序坞",
        "SystemUIServer",
        "Control Center",
        "控制中心",
        "Spotlight",
        "聚焦",
        "NotificationCenter",
        "通知中心",
        "Finder",
        "访达",
        "TextInputMenuAgent",
        "Wallpaper",
        "WindowManager",
        "AirPlayUIAgent",
        "Siri",
        "loginwindow",
        "ControlStrip",
        "CoreServicesUIAgent",
        "ScreenSaverEngine",
        "universalAccessAuthWarn",
    ];

    // 已知会产生无用浮动工具栏/面板的应用
    // 这些应用的浮动窗口（非前台时）几乎一定是悬浮工具栏，不应计为独立使用时长
    const TOOLBAR_APPS: &[&str] = &[
        "WPS Office",
        "wpsoffice",
        "WPS",
        "Microsoft Word",
        "Microsoft Excel",
        "Microsoft PowerPoint",
    ];

    let mut results: Vec<ActiveWindow> = Vec::new();

    unsafe {
        let window_list = CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            kCGNullWindowID,
        );
        if window_list.is_null() {
            return results;
        }

        let count = CFArrayGetCount(window_list as _);

        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(window_list as _, i) as CFDictionaryRef;
            if dict.is_null() {
                continue;
            }

            // 读取 kCGWindowLayer
            let layer_key = CFString::new("kCGWindowLayer");
            let mut layer_ref: CFTypeRef = std::ptr::null();
            if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict,
                layer_key.as_CFTypeRef() as *const _,
                &mut layer_ref,
            ) == 0
                || layer_ref.is_null()
            {
                continue;
            }
            let mut layer: i32 = 0;
            if !core_foundation::number::CFNumberGetValue(
                layer_ref as CFNumberRef,
                core_foundation::number::kCFNumberSInt32Type,
                &mut layer as *mut i32 as *mut _,
            ) {
                continue;
            }

            // 只取浮动窗口 (layer > 0)
            if layer <= 0 {
                continue;
            }

            // 读取 kCGWindowOwnerName
            let owner_key = CFString::new("kCGWindowOwnerName");
            let mut owner_ref: CFTypeRef = std::ptr::null();
            if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict,
                owner_key.as_CFTypeRef() as *const _,
                &mut owner_ref,
            ) == 0
                || owner_ref.is_null()
            {
                continue;
            }
            let owner_cfstr =
                core_foundation::string::CFString::wrap_under_get_rule(owner_ref as _);
            let owner_name = owner_cfstr.to_string();

            // 排除当前前台应用（避免重复计时）
            if owner_name == frontmost_app {
                continue;
            }

            // 排除系统进程（使用包含匹配，兼容中英文系统名称差异）
            if SYSTEM_PROCESSES
                .iter()
                .any(|&sys| owner_name == sys || owner_name.contains(sys))
            {
                continue;
            }

            // 排除已知悬浮工具栏应用的浮动窗口
            if TOOLBAR_APPS.iter().any(|&app| owner_name.contains(app)) {
                log::debug!("🪟 排除工具栏浮动窗口: {} (layer={})", owner_name, layer);
                continue;
            }

            // 读取窗口尺寸 kCGWindowBounds
            let bounds_key = CFString::new("kCGWindowBounds");
            let mut bounds_ref: CFTypeRef = std::ptr::null();
            if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict,
                bounds_key.as_CFTypeRef() as *const _,
                &mut bounds_ref,
            ) == 0
                || bounds_ref.is_null()
            {
                continue;
            }
            // kCGWindowBounds 是一个 CFDictionary: {Height, Width, X, Y}
            let bounds_dict = bounds_ref as CFDictionaryRef;

            let width = get_cf_dict_number(bounds_dict, "Width").unwrap_or(0.0);
            let height = get_cf_dict_number(bounds_dict, "Height").unwrap_or(0.0);

            // 排除小图标/指示器/工具栏类窗口
            // WPS Office 等应用常驻的悬浮工具栏尺寸较小，需要提高阈值
            if width <= 200.0 || height <= 150.0 {
                continue;
            }

            // 读取 kCGWindowName（可选）
            let win_name_key = CFString::new("kCGWindowName");
            let mut win_name_ref: CFTypeRef = std::ptr::null();
            let window_title = if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
                dict,
                win_name_key.as_CFTypeRef() as *const _,
                &mut win_name_ref,
            ) != 0
                && !win_name_ref.is_null()
            {
                let name_cfstr =
                    core_foundation::string::CFString::wrap_under_get_rule(win_name_ref as _);
                name_cfstr.to_string()
            } else {
                String::new()
            };

            // 无窗口标题的浮动窗口大概率是工具栏/面板/悬浮球，用更严格的阈值
            if window_title.is_empty() && (width <= 400.0 || height <= 300.0) {
                continue;
            }

            log::debug!(
                "🪟 检测到浮动窗口: {} - {} (layer={}, {}x{})",
                owner_name,
                window_title,
                layer,
                width as i32,
                height as i32
            );

            results.push(ActiveWindow {
                app_name: owner_name,
                window_title,
                browser_url: None,
            });
        }

        CFRelease(window_list as _);
    }

    // 去重：同一应用可能有多个浮动窗口，只保留第一个
    results.dedup_by(|a, b| a.app_name == b.app_name);

    results
}

/// 从 CFDictionary 读取一个数值字段
#[cfg(target_os = "macos")]
unsafe fn get_cf_dict_number(
    dict: core_foundation::dictionary::CFDictionaryRef,
    key: &str,
) -> Option<f64> {
    use core_foundation::base::{CFTypeRef, TCFType};
    use core_foundation::string::CFString;

    let cf_key = CFString::new(key);
    let mut val_ref: CFTypeRef = std::ptr::null();
    if core_foundation::dictionary::CFDictionaryGetValueIfPresent(
        dict,
        cf_key.as_CFTypeRef() as *const _,
        &mut val_ref,
    ) == 0
        || val_ref.is_null()
    {
        return None;
    }
    let mut value: f64 = 0.0;
    if core_foundation::number::CFNumberGetValue(
        val_ref as core_foundation::number::CFNumberRef,
        core_foundation::number::kCFNumberFloat64Type,
        &mut value as *mut f64 as *mut _,
    ) {
        Some(value)
    } else {
        None
    }
}

/// 非 macOS 平台：返回空 Vec
#[cfg(not(target_os = "macos"))]
pub fn get_overlay_windows(_frontmost_app: &str) -> Vec<ActiveWindow> {
    Vec::new()
}

/// 获取所有可见窗口 (macOS)
/// 当前为预留功能
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn get_visible_windows() -> Result<Vec<ActiveWindow>> {
    use std::process::Command;

    // 使用 AppleScript 获取所有可见窗口
    let script = r#"
        set output to ""
        tell application "System Events"
            set allProcesses to every process whose visible is true
            repeat with proc in allProcesses
                try
                    set procName to name of proc
                    set windowList to every window of proc
                    repeat with win in windowList
                        try
                            set winName to name of win
                            set output to output & procName & "|" & winName & linefeed
                        end try
                    end repeat
                end try
            end repeat
        end tell
        return output
    "#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| AppError::Screenshot(format!("执行AppleScript失败: {e}")))?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        let windows: Vec<ActiveWindow> = result
            .lines()
            .filter(|line| !line.is_empty())
            .take(10) // 最多10个窗口
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                let app_name = parts.first().unwrap_or(&"Unknown").to_string();
                let window_title = parts.get(1).unwrap_or(&"").to_string();
                let browser_url = get_browser_url(&app_name);
                ActiveWindow {
                    app_name,
                    window_title,
                    browser_url,
                }
            })
            .collect();
        Ok(windows)
    } else {
        Ok(vec![])
    }
}

/// 获取所有可见窗口 (非 macOS)
#[cfg(not(target_os = "macos"))]
pub fn get_visible_windows() -> Result<Vec<ActiveWindow>> {
    // 非 macOS 平台暂不支持多窗口
    get_active_window().map(|w| vec![w])
}

/// 根据应用名自动分类
pub fn categorize_app(app_name: &str, window_title: &str) -> String {
    let app_lower = app_name.to_lowercase();

    // 开发工具（IDE、编辑器、终端、数据库工具、API 工具、容器、版本控制）
    if app_lower.contains("code")
        || app_lower.contains("visual studio")
        || app_lower.contains("cursor")
        || app_lower.contains("idea")
        || app_lower.contains("pycharm")
        || app_lower.contains("webstorm")
        || app_lower.contains("goland")
        || app_lower.contains("clion")
        || app_lower.contains("rustrover")
        || app_lower.contains("rider")
        || app_lower.contains("phpstorm")
        || app_lower.contains("datagrip")
        || app_lower.contains("fleet")
        || app_lower.contains("xcode")
        || app_lower.contains("android studio")
        || app_lower.contains("hbuilder")
        || app_lower.contains("sublime")
        || app_lower.contains("atom")
        || app_lower.contains("vim")
        || app_lower.contains("neovim")
        || app_lower.contains("emacs")
        || app_lower.contains("nova")
        || app_lower.contains("bbedit")
        || app_lower.contains("coteditor")
        || app_lower.contains("textmate")
        || app_lower.contains("terminal")
        || app_lower.contains("iterm")
        || app_lower.contains("warp")
        || app_lower.contains("alacritty")
        || app_lower.contains("kitty")
        || app_lower.contains("wezterm")
        || app_lower.contains("hyper")
        || app_lower.contains("windowsterminal")
        || app_lower.contains("cmd")
        || app_lower.contains("powershell")
        || app_lower.contains("git")
        || app_lower.contains("sourcetree")
        || app_lower.contains("gitkraken")
        || app_lower.contains("docker")
        || app_lower.contains("postman")
        || app_lower.contains("insomnia")
        || app_lower.contains("dbeaver")
        || app_lower.contains("navicat")
        || app_lower.contains("tableplus")
        || app_lower.contains("sequel")
        || app_lower.contains("charles")
        || app_lower.contains("fiddler")
    {
        return "development".to_string();
    }

    // 浏览器（支持市面上所有主流浏览器，包含 Windows 进程名）
    // 注意：短名称用精确匹配或 starts_with，避免误匹配系统进程
    if app_lower.contains("chrome")
        || app_lower.contains("firefox")
        || app_lower.contains("safari")
        || app_lower.contains("msedge")
        || app_lower.contains("opera")
        || app_lower.contains("brave")
        || app_lower.starts_with("arc")
        || app_lower.contains("vivaldi")
        || app_lower.contains("chromium")
        || app_lower.contains("orion")
        || app_lower.starts_with("zen")
        || app_lower.contains("sidekick")
        || app_lower.contains("wavebox")
        || app_lower.contains("maxthon")
        || app_lower.contains("waterfox")
        || app_lower.contains("librewolf")
        || app_lower.contains("tor browser")
        || app_lower.contains("duckduckgo")
        || app_lower.contains("yandex")
        || app_lower.starts_with("whale")
        || app_lower.contains("naver")
        || app_lower.contains("uc browser")
        || app_lower.contains("qqbrowser")
        || app_lower.contains("360se")
        || app_lower.contains("360chrome")
        || app_lower.contains("sogouexplorer")
        || app_lower.contains("2345explorer")
        || app_lower.contains("liebao")
        || app_lower.contains("theworld")
        || app_lower.contains("centbrowser")
        || app_lower.contains("iexplore")
        || app_lower.contains("qq浏览器")
        || app_lower.contains("360浏览器")
        || app_lower.contains("搜狗浏览器")
    {
        return "browser".to_string();
    }

    // 通讯工具（注意：qq 的匹配要排除已被浏览器捕获的 qqbrowser）
    if app_lower.contains("slack")
        || app_lower.contains("teams")
        || app_lower.contains("zoom")
        || app_lower.contains("discord")
        || app_lower.contains("wechat")
        || app_lower.contains("微信")
        || app_lower.contains("wecom")
        || app_lower.contains("企业微信")
        || (app_lower.contains("qq") && !app_lower.contains("qqbrowser"))
        || app_lower.contains("telegram")
        || app_lower.contains("skype")
        || app_lower.contains("dingtalk")
        || app_lower.contains("钉钉")
        || app_lower.contains("飞书")
        || app_lower.contains("lark")
    {
        return "communication".to_string();
    }

    // 办公软件
    if app_lower.contains("word")
        || app_lower.contains("excel")
        || app_lower.contains("powerpoint")
        || app_lower.contains("pages")
        || app_lower.contains("numbers")
        || app_lower.contains("keynote")
        || app_lower.contains("notion")
        || app_lower.contains("obsidian")
        || app_lower.contains("logseq")
        || app_lower.contains("evernote")
        || app_lower.contains("onenote")
        || app_lower.contains("wps")
        || app_lower.contains("typora")
        || app_lower.contains("bear")
        || app_lower.contains("ulysses")
        || app_lower.contains("xmind")
        || app_lower.contains("mindnode")
    {
        return "office".to_string();
    }

    // 设计工具
    if app_lower.contains("figma")
        || app_lower.contains("sketch")
        || app_lower.contains("photoshop")
        || app_lower.contains("illustrator")
        || app_lower.contains("xd")
        || app_lower.contains("canva")
        || app_lower.contains("pixelmator")
        || app_lower.contains("affinity")
        || app_lower.contains("lightroom")
        || app_lower.contains("indesign")
    {
        return "design".to_string();
    }

    // 娱乐
    if app_lower.contains("spotify")
        || app_lower.contains("music")
        || app_lower.contains("youtube")
        || app_lower.contains("netflix")
        || app_lower.contains("bilibili")
        || app_lower.contains("game")
        || app_lower.contains("steam")
        || app_lower.contains("网易云")
        || app_lower.contains("qqmusic")
        || app_lower.contains("爱奇艺")
    {
        return "entertainment".to_string();
    }

    // 窗口标题兜底：app_name 无法识别时，用窗口标题中的 IDE/工具关键词做最后一轮匹配
    // 典型场景：Windows 上 JetBrains IDE 进程名可能是 java.exe / idea64.exe 截断后不匹配
    if !window_title.is_empty() {
        let title_lower = window_title.to_lowercase();
        if title_lower.contains("intellij")
            || title_lower.contains("pycharm")
            || title_lower.contains("webstorm")
            || title_lower.contains("goland")
            || title_lower.contains("clion")
            || title_lower.contains("datagrip")
            || title_lower.contains("rustrover")
            || title_lower.contains("visual studio")
            || title_lower.contains("vs code")
            || title_lower.contains("cursor")
        {
            return "development".to_string();
        }
    }

    "other".to_string()
}

/// 获取分类的中文名称
pub fn get_category_name(category: &str) -> &str {
    match category {
        "development" => "开发工具",
        "browser" => "浏览器",
        "communication" => "通讯协作",
        "office" => "办公软件",
        "design" => "设计工具",
        "entertainment" => "娱乐",
        _ => "其他",
    }
}

/// 获取分类的图标
#[allow(dead_code)]
pub fn get_category_icon(category: &str) -> &str {
    match category {
        "development" => "💻",
        "browser" => "🌐",
        "communication" => "💬",
        "office" => "📄",
        "design" => "🎨",
        "entertainment" => "🎵",
        _ => "📦",
    }
}
