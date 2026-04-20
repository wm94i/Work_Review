#[cfg(windows)]
use auto_launch::{AutoLaunch, AutoLaunchBuilder, WindowsEnableMode};
#[cfg(windows)]
use std::env::current_exe;
use tauri::AppHandle;
#[cfg(windows)]
use tauri::Manager;
#[cfg(not(windows))]
use tauri_plugin_autostart::ManagerExt;

use crate::error::{AppError, Result};

#[cfg(windows)]
const AUTOSTART_LAUNCH_ARG: &str = "--autostart";
#[cfg(windows)]
const AUTOSTART_HIDDEN_ARG: &str = "--hidden";
#[cfg(windows)]
const AUTOSTART_NOT_INITIALIZED_ERROR: &str = "开机自启动服务未初始化，请重启应用后重试";

/// Windows 上保存构造 AutoLaunch 所需的稳定字段，enable/disable/is_enabled
/// 都据此按需重建 builder。这样 silent 状态变化时能即时写入新的 launch args，
/// 不再把 `--hidden` 硬编码到 init。
#[cfg(windows)]
pub struct AutoLaunchConfig {
    app_name: String,
    quoted_app_path: String,
}

#[cfg(windows)]
fn quote_windows_command_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"');
    format!("\"{trimmed}\"")
}

#[cfg(windows)]
fn autolaunch_args(silent: bool) -> &'static [&'static str] {
    if silent {
        &[AUTOSTART_LAUNCH_ARG, AUTOSTART_HIDDEN_ARG]
    } else {
        &[AUTOSTART_LAUNCH_ARG]
    }
}

#[cfg(windows)]
fn build_autolaunch(config: &AutoLaunchConfig, silent: bool) -> Result<AutoLaunch> {
    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(&config.app_name);
    builder.set_app_path(&config.quoted_app_path);
    builder.set_windows_enable_mode(WindowsEnableMode::Dynamic);
    builder.set_args(autolaunch_args(silent));
    builder
        .build()
        .map_err(|e| AppError::Unknown(format!("构建 Windows 开机自启管理器失败: {e}")))
}

#[cfg(windows)]
fn with_windows_config<T>(
    app: &AppHandle,
    operation: impl FnOnce(&AutoLaunchConfig) -> Result<T>,
) -> Result<T> {
    let config = app
        .try_state::<AutoLaunchConfig>()
        .ok_or_else(|| AppError::Unknown(AUTOSTART_NOT_INITIALIZED_ERROR.to_string()))?;
    operation(&config)
}

#[cfg(windows)]
pub fn init_autostart(app: &AppHandle) -> Result<()> {
    let current_exe =
        current_exe().map_err(|e| AppError::Unknown(format!("无法获取当前执行路径: {e}")))?;
    let quoted_exe = quote_windows_command_path(&current_exe.display().to_string());

    app.manage(AutoLaunchConfig {
        app_name: app.package_info().name.clone(),
        quoted_app_path: quoted_exe,
    });
    Ok(())
}

#[cfg(not(windows))]
pub fn init_autostart(_app: &AppHandle) -> Result<()> {
    Ok(())
}

#[tauri::command]
pub fn enable_autostart(app: AppHandle, silent: bool) -> Result<()> {
    #[cfg(windows)]
    {
        return with_windows_config(&app, |config| {
            let auto = build_autolaunch(config, silent)?;
            auto.enable()
                .map_err(|e| AppError::Unknown(format!("开启开机自启失败: {e}")))?;
            Ok(())
        });
    }

    #[cfg(not(windows))]
    {
        // macOS/Linux: plugin 初始化时已固定注入 `--autostart --hidden`，
        // silent 由前端 config 主导，这里收下参数保持与 Windows 对齐但不使用。
        let _ = silent;
        app.autolaunch()
            .enable()
            .map_err(|e| AppError::Unknown(format!("开启开机自启失败: {e}")))?;
        Ok(())
    }
}

#[tauri::command]
pub fn disable_autostart(app: AppHandle) -> Result<()> {
    #[cfg(windows)]
    {
        return with_windows_config(&app, |config| {
            let auto = build_autolaunch(config, false)?;
            auto.disable()
                .map_err(|e| AppError::Unknown(format!("关闭开机自启失败: {e}")))?;
            Ok(())
        });
    }

    #[cfg(not(windows))]
    {
        app.autolaunch()
            .disable()
            .map_err(|e| AppError::Unknown(format!("关闭开机自启失败: {e}")))?;
        Ok(())
    }
}

#[tauri::command]
pub fn is_autostart_enabled(app: AppHandle) -> Result<bool> {
    #[cfg(windows)]
    {
        return with_windows_config(&app, |config| {
            let auto = build_autolaunch(config, false)?;
            auto.is_enabled()
                .map_err(|e| AppError::Unknown(format!("获取开机自启状态失败: {e}")))
        });
    }

    #[cfg(not(windows))]
    {
        app.autolaunch()
            .is_enabled()
            .map_err(|e| AppError::Unknown(format!("获取开机自启状态失败: {e}")))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::{autolaunch_args, quote_windows_command_path, AUTOSTART_HIDDEN_ARG};

    #[test]
    fn windows自启动路径应始终带引号() {
        assert_eq!(
            quote_windows_command_path(r#"C:\Program Files\Work Review\work-review.exe"#),
            r#""C:\Program Files\Work Review\work-review.exe""#
        );
        assert_eq!(
            quote_windows_command_path(r#""C:\Work Review\work-review.exe""#),
            r#""C:\Work Review\work-review.exe""#
        );
    }

    #[test]
    fn 静默模式应额外注入_hidden_参数供开机时拉起隐藏窗口() {
        let silent = autolaunch_args(true);
        assert!(silent.iter().any(|arg| *arg == AUTOSTART_HIDDEN_ARG));

        let visible = autolaunch_args(false);
        assert!(!visible.iter().any(|arg| *arg == AUTOSTART_HIDDEN_ARG));
    }
}
