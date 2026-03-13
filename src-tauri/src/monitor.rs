use crate::error::{AppError, Result};

/// 活动窗口信息
#[derive(Debug, Clone)]
pub struct ActiveWindow {
    pub app_name: String,
    pub window_title: String,
    /// 浏览器 URL（如果当前应用是浏览器）
    pub browser_url: Option<String>,
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
        let browser_url = get_browser_url_windows(&app_name, &window_title);

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

/// 从窗口标题提取浏览器 URL (Windows)
/// 大多数浏览器会在标题栏显示页面标题，部分会包含 URL
/// 使用缓存避免频繁启动 PowerShell 进程（UI Automation 操作较重）
#[cfg(target_os = "windows")]
fn get_browser_url_windows(app_name: &str, window_title: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    use std::sync::Mutex;
    use std::time::Instant;

    // 缓存结构：(app_name, window_title, cached_url, fetch_time)
    static URL_CACHE: std::sync::LazyLock<Mutex<(String, String, Option<String>, Instant)>> =
        std::sync::LazyLock::new(|| Mutex::new((String::new(), String::new(), None, Instant::now())));

    const CACHE_TTL_SECS: u64 = 30;

    // 检查缓存：同一浏览器+同一标题（标签页未切换）且未过期，直接返回
    if let Ok(cache) = URL_CACHE.lock() {
        if cache.0 == app_name
            && cache.1 == window_title
            && cache.3.elapsed().as_secs() < CACHE_TTL_SECS
        {
            return cache.2.clone();
        }
    }

    let app_lower = app_name.to_lowercase();

    // 检查是否为浏览器进程（包括国产浏览器）
    let is_chromium_based = app_lower.contains("chrome")
        || app_lower.contains("msedge")
        || app_lower.contains("brave")
        || app_lower.contains("opera")
        || app_lower.contains("vivaldi")
        || app_lower.contains("360se")      // 360 安全浏览器
        || app_lower.contains("360chrome")  // 360 极速浏览器
        || app_lower.contains("qqbrowser")  // QQ 浏览器
        || app_lower.contains("sogouexplorer") // 搜狗浏览器
        || app_lower.contains("2345explorer") // 2345 浏览器
        || app_lower.contains("liebao")     // 猎豹浏览器
        || app_lower.contains("maxthon")    // 傲游浏览器
        || app_lower.contains("theworld")   // 世界之窗
        || app_lower.contains("cent");      // Cent Browser

    let is_firefox = app_lower.contains("firefox");
    let is_ie = app_lower.contains("iexplore");

    if !is_chromium_based && !is_firefox && !is_ie {
        return None;
    }

    // CREATE_NO_WINDOW 标志，防止弹出黑色控制台窗口
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // 尝试使用 PowerShell 和 UI Automation 获取 URL
    // 这是一个较重的操作，仅在确认是浏览器时执行
    use std::process::Command;

    // 根据不同浏览器使用不同的方法
    // Chromium 系浏览器使用相同的 UI 结构
    let script = if is_chromium_based {
        // Chrome/Edge/Brave 使用类似的 UI 结构
        r#"
        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        
        $root = [System.Windows.Automation.AutomationElement]::RootElement
        $condition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit
        )
        
        # 获取前台窗口
        $foreground = [System.Windows.Automation.AutomationElement]::FocusedElement
        if ($foreground -eq $null) { exit }
        
        # 向上查找顶层窗口
        $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
        $window = $foreground
        while ($window.Current.ControlType -ne [System.Windows.Automation.ControlType]::Window -and $window -ne $root) {
            $window = $walker.GetParent($window)
        }
        
        if ($window -eq $null -or $window -eq $root) { exit }
        
        # 查找地址栏 (Edit 控件)
        $edits = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
        foreach ($edit in $edits) {
            try {
                $pattern = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
                $value = $pattern.Current.Value
                if ($value -match '^https?://' -or $value -match '^[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}') {
                    Write-Output $value
                    exit
                }
            } catch { }
        }
        "#
    } else if is_firefox {
        // Firefox 的 UI 结构略有不同
        r#"
        Add-Type -AssemblyName UIAutomationClient
        Add-Type -AssemblyName UIAutomationTypes
        
        $root = [System.Windows.Automation.AutomationElement]::RootElement
        
        # 查找 Firefox 窗口
        $ffCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ClassNameProperty, "MozillaWindowClass"
        )
        $ffWindow = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $ffCondition)
        
        if ($ffWindow -eq $null) { exit }
        
        # 查找 URL 栏
        $editCondition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit
        )
        $edits = $ffWindow.FindAll([System.Windows.Automation.TreeScope]::Descendants, $editCondition)
        
        foreach ($edit in $edits) {
            try {
                $pattern = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
                $value = $pattern.Current.Value
                if ($value -match '^https?://' -or $value -match '^[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}') {
                    Write-Output $value
                    exit
                }
            } catch { }
        }
        "#
    } else {
        return None;
    };

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    let result = if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            // 如果不是完整 URL，添加 https:// 前缀
            if url.starts_with("http://") || url.starts_with("https://") {
                Some(url)
            } else if url.contains('.') && !url.contains(' ') {
                Some(format!("https://{}", url))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // 更新缓存
    if let Ok(mut cache) = URL_CACHE.lock() {
        *cache = (
            app_name.to_string(),
            window_title.to_string(),
            result.clone(),
            Instant::now(),
        );
    }

    result
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
        return URL of current tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
        return URL of active tab of window 1
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
pub fn categorize_app(app_name: &str) -> String {
    let app_lower = app_name.to_lowercase();

    // 开发工具
    if app_lower.contains("code")
        || app_lower.contains("visual studio")
        || app_lower.contains("idea")
        || app_lower.contains("pycharm")
        || app_lower.contains("webstorm")
        || app_lower.contains("xcode")
        || app_lower.contains("android studio")
        || app_lower.contains("sublime")
        || app_lower.contains("atom")
        || app_lower.contains("vim")
        || app_lower.contains("emacs")
        || app_lower.contains("terminal")
        || app_lower.contains("iterm")
        || app_lower.contains("cmd")
        || app_lower.contains("powershell")
        || app_lower.contains("git")
    {
        return "development".to_string();
    }

    // 浏览器（支持市面上所有主流浏览器，包含 Windows 进程名）
    // 注意：短名称用精确匹配或 starts_with，避免误匹配系统进程
    // 例如 "arc" 会匹配 SearchHost（s-e-a-r-c-h），"cent" 会匹配 recent
    if app_lower.contains("chrome")
        || app_lower.contains("firefox")
        || app_lower.contains("safari")
        || app_lower.contains("msedge")        // Microsoft Edge (进程名 msedge.exe)
        || app_lower.contains("opera")
        || app_lower.contains("brave")
        || app_lower.starts_with("arc")         // Arc 浏览器（避免匹配 searchhost 等）
        || app_lower.contains("vivaldi")
        || app_lower.contains("chromium")
        || app_lower.contains("orion")
        || app_lower.starts_with("zen")         // Zen Browser（避免匹配 citizen 等）
        || app_lower.contains("sidekick")
        || app_lower.contains("wavebox")
        || app_lower.contains("maxthon")
        || app_lower.contains("waterfox")
        || app_lower.contains("librewolf")
        || app_lower.contains("tor browser")
        || app_lower.contains("duckduckgo")
        || app_lower.contains("yandex")
        || app_lower.starts_with("whale")       // Naver Whale（避免误匹配）
        || app_lower.contains("naver")
        || app_lower.contains("uc browser")
        // Windows 进程名（英文）
        || app_lower.contains("qqbrowser")       // QQ 浏览器
        || app_lower.contains("360se")           // 360 安全浏览器
        || app_lower.contains("360chrome")       // 360 极速浏览器
        || app_lower.contains("sogouexplorer")   // 搜狗浏览器
        || app_lower.contains("2345explorer")    // 2345 浏览器
        || app_lower.contains("liebao")          // 猎豹浏览器
        || app_lower.contains("theworld")        // 世界之窗
        || app_lower.contains("centbrowser")     // Cent Browser（精确匹配进程名）
        || app_lower.contains("iexplore")        // IE 浏览器
        // 中文显示名（macOS / 窗口标题匹配）
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
        || app_lower.contains("evernote")
        || app_lower.contains("onenote")
        || app_lower.contains("wps")
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
    {
        return "entertainment".to_string();
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
