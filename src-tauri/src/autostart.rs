use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use std::env::current_exe;
use tauri::{AppHandle, Manager, Runtime};

const AUTOSTART_LAUNCH_ARG: &str = "--autostart";

pub struct AutoLaunchManager(AutoLaunch);

pub fn init_autostart(app: &AppHandle) -> Result<(), crate::error::AppError> {
    let mut builder = AutoLaunchBuilder::new();

    let app_name = &app.package_info().name;
    builder.set_app_name(app_name);

    builder.set_args(&[AUTOSTART_LAUNCH_ARG]);

    let current_exe = current_exe()
        .map_err(|e| crate::error::AppError::Unknown(format!("无法获取当前执行路径: {e}")))?;

    #[cfg(windows)]
    builder.set_app_path(&current_exe.display().to_string());

// 未实现macos和linux的开机自启特殊处理

    let auto = builder.build()
        .map_err(|e| crate::error::AppError::Unknown(format!("构建 AutoLaunch 失败: {e}")))?;

    app.manage(AutoLaunchManager(auto));
    Ok(())
}

#[tauri::command]
pub fn enable_autostart(manager: tauri::State<'_, AutoLaunchManager>) -> Result<(), crate::error::AppError> {
    manager.0.enable()
        .map_err(|e| crate::error::AppError::Unknown(format!("开启开机自启失败: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn disable_autostart(manager: tauri::State<'_, AutoLaunchManager>) -> Result<(), crate::error::AppError> {
    manager.0.disable()
        .map_err(|e| crate::error::AppError::Unknown(format!("关闭开机自启失败: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn is_autostart_enabled(manager: tauri::State<'_, AutoLaunchManager>) -> Result<bool, crate::error::AppError> {
    manager.0.is_enabled()
        .map_err(|e| crate::error::AppError::Unknown(format!("获取开机自启状态失败: {e}")))
}