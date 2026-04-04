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

const AUTOSTART_LAUNCH_ARG: &str = "--autostart";
#[cfg(windows)]
const AUTOSTART_NOT_INITIALIZED_ERROR: &str = "开机自启动服务未初始化，请重启应用后重试";

#[cfg(windows)]
pub struct AutoLaunchManager(AutoLaunch);

#[cfg(windows)]
pub fn init_autostart(app: &AppHandle) -> Result<()> {
    let current_exe =
        current_exe().map_err(|e| AppError::Unknown(format!("无法获取当前执行路径: {e}")))?;

    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(&app.package_info().name);
    builder.set_app_path(&current_exe.display().to_string());
    builder.set_windows_enable_mode(WindowsEnableMode::Dynamic);
    builder.set_args(&[AUTOSTART_LAUNCH_ARG]);

    let auto = builder
        .build()
        .map_err(|e| AppError::Unknown(format!("构建 Windows 开机自启管理器失败: {e}")))?;

    app.manage(AutoLaunchManager(auto));
    Ok(())
}

#[cfg(not(windows))]
pub fn init_autostart(_app: &AppHandle) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
fn with_windows_manager<T>(
    app: &AppHandle,
    operation: impl FnOnce(&AutoLaunchManager) -> Result<T>,
) -> Result<T> {
    let manager = app
        .try_state::<AutoLaunchManager>()
        .ok_or_else(|| AppError::Unknown(AUTOSTART_NOT_INITIALIZED_ERROR.to_string()))?;

    operation(&manager)
}

#[tauri::command]
pub fn enable_autostart(app: AppHandle) -> Result<()> {
    #[cfg(windows)]
    {
        return with_windows_manager(&app, |manager| {
            manager
                .0
                .enable()
                .map_err(|e| AppError::Unknown(format!("开启开机自启失败: {e}")))?;
            Ok(())
        });
    }

    #[cfg(not(windows))]
    {
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
        return with_windows_manager(&app, |manager| {
            manager
                .0
                .disable()
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
        return with_windows_manager(&app, |manager| {
            manager
                .0
                .is_enabled()
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
