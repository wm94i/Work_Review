use crate::error::{AppError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use image::{imageops::FilterType, ColorType, DynamicImage};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// 检查 macOS 屏幕录制权限（不触发授权弹窗）
/// 使用 CGPreflightScreenCaptureAccess (macOS 10.15+)
/// 返回 true 表示已授权，false 表示未授权
#[cfg(target_os = "macos")]
pub fn has_screen_capture_permission() -> bool {
    // CGPreflightScreenCaptureAccess: 仅检查，不弹窗
    // macOS 10.15+ 提供，10.14 及以下默认返回 true（不需要权限）
    extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
    }
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// 请求 macOS 屏幕录制权限（触发系统弹窗引导用户到设置）
/// 使用 CGRequestScreenCaptureAccess (macOS 10.15+)
/// 注意：此函数仅触发系统提示，用户需要手动在系统设置中授权后重启应用
#[cfg(target_os = "macos")]
pub fn request_screen_capture_permission() {
    extern "C" {
        fn CGRequestScreenCaptureAccess() -> bool;
    }
    unsafe {
        CGRequestScreenCaptureAccess();
    }
}

/// 检查 macOS 辅助功能（Accessibility）权限
/// AppleScript 读取窗口标题、浏览器 URL 均需要此权限
/// prompt=true 时弹出系统授权引导
#[cfg(target_os = "macos")]
pub fn has_accessibility_permission(prompt: bool) -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    extern "C" {
        fn AXIsProcessTrustedWithOptions(
            options: core_foundation::dictionary::CFDictionaryRef,
        ) -> bool;
    }

    if prompt {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key, value)]);
        unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
    } else {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::false_value();
        let options = CFDictionary::from_CFType_pairs(&[(key, value)]);
        unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn has_screen_capture_permission() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn has_accessibility_permission(_prompt: bool) -> bool {
    true
}

/// 截屏结果
#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    pub path: PathBuf,
    pub timestamp: i64,
    pub width: u32,
    pub height: u32,
}

/// 截屏服务配置
pub struct ScreenshotConfig {
    /// 最大宽度（超过此宽度会按比例缩放）
    pub max_width: u32,
    /// JPEG 质量 (1-100)
    pub jpeg_quality: u8,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            max_width: 1440,
            jpeg_quality: 70,
        }
    }
}

/// 截屏服务
pub struct ScreenshotService {
    data_dir: PathBuf,
    config: ScreenshotConfig,
}

impl ScreenshotService {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            config: ScreenshotConfig::default(),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn capture_for_window(
        &self,
        active_window: Option<&crate::monitor::ActiveWindow>,
    ) -> Result<ScreenshotResult> {
        self.capture_impl(active_window)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub fn capture_for_window(
        &self,
        _active_window: Option<&crate::monitor::ActiveWindow>,
    ) -> Result<ScreenshotResult> {
        self.capture_impl()
    }

    pub fn capture(&self) -> Result<ScreenshotResult> {
        self.capture_for_window(None)
    }

    /// 执行截屏（Windows）
    /// 优先使用 Windows Graphics Capture API (Win11)
    /// 失败时降级使用 GDI BitBlt (Win10 兼容)
    #[cfg(target_os = "windows")]
    fn capture_impl(
        &self,
        active_window: Option<&crate::monitor::ActiveWindow>,
    ) -> Result<ScreenshotResult> {
        // 生成文件路径
        let now = chrono::Local::now();
        let date_str = now.format("%Y-%m-%d").to_string();
        let time_str = now.format("%H%M%S_%3f").to_string();

        let screenshots_dir = self.data_dir.join("screenshots").join(&date_str);
        std::fs::create_dir_all(&screenshots_dir)?;

        let final_jpg = screenshots_dir.join(format!("{time_str}.jpg"));

        // 先尝试 Windows Graphics Capture API
        match self.capture_with_wgc(&screenshots_dir, &time_str, active_window) {
            Ok(result) => {
                return self.process_and_save_image(
                    &result.0,
                    &final_jpg,
                    result.1,
                    result.2,
                    now.timestamp(),
                );
            }
            Err(e) => {
                log::warn!("Windows Graphics Capture 失败: {e}，降级到 GDI 模式");
            }
        }

        // 降级使用 GDI BitBlt（Windows 10 兼容方案）
        match self.capture_with_gdi(active_window) {
            Ok((pixels, width, height)) => {
                self.save_rgba_to_jpeg(&pixels, width, height, &final_jpg, now.timestamp())
            }
            Err(e) => Err(AppError::Screenshot(format!("GDI 截图也失败: {e}"))),
        }
    }

    /// 使用 Windows Graphics Capture API 截屏
    #[cfg(target_os = "windows")]
    fn capture_with_wgc(
        &self,
        screenshots_dir: &Path,
        time_str: &str,
        active_window: Option<&crate::monitor::ActiveWindow>,
    ) -> Result<(PathBuf, u32, u32)> {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        };
        use windows_capture::{
            capture::GraphicsCaptureApiHandler,
            frame::Frame,
            graphics_capture_api::InternalCaptureControl,
            monitor::Monitor,
            settings::{
                ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
                MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
            },
        };

        let temp_png = screenshots_dir.join(format!("{time_str}_temp.png"));

        struct CaptureResult {
            success: bool,
            error: Option<String>,
            width: u32,
            height: u32,
        }

        let result = Arc::new(Mutex::new(CaptureResult {
            success: false,
            error: None,
            width: 0,
            height: 0,
        }));
        let captured = Arc::new(AtomicBool::new(false));

        struct SingleFrameCapture {
            result: Arc<Mutex<CaptureResult>>,
            captured: Arc<AtomicBool>,
            output_path: PathBuf,
        }

        impl GraphicsCaptureApiHandler for SingleFrameCapture {
            type Flags = (Arc<Mutex<CaptureResult>>, Arc<AtomicBool>, PathBuf);
            type Error = Box<dyn std::error::Error + Send + Sync>;

            fn new(
                ctx: windows_capture::capture::Context<Self::Flags>,
            ) -> std::result::Result<Self, Self::Error> {
                Ok(Self {
                    result: ctx.flags.0,
                    captured: ctx.flags.1,
                    output_path: ctx.flags.2,
                })
            }

            fn on_frame_arrived(
                &mut self,
                frame: &mut Frame,
                capture_control: InternalCaptureControl,
            ) -> std::result::Result<(), Self::Error> {
                if self.captured.load(Ordering::SeqCst) {
                    capture_control.stop();
                    return Ok(());
                }

                self.captured.store(true, Ordering::SeqCst);

                let width = frame.width();
                let height = frame.height();

                use windows_capture::frame::ImageFormat;
                match frame.save_as_image(&self.output_path, ImageFormat::Png) {
                    Ok(()) => {
                        if let Ok(mut r) = self.result.lock() {
                            r.success = true;
                            r.width = width;
                            r.height = height;
                        }
                    }
                    Err(e) => {
                        if let Ok(mut r) = self.result.lock() {
                            r.error = Some(format!("{}", e));
                        }
                    }
                }

                capture_control.stop();
                Ok(())
            }

            fn on_closed(&mut self) -> std::result::Result<(), Self::Error> {
                Ok(())
            }
        }

        let target_monitor = capture_target_monitor(active_window)
            .or_else(|| Monitor::primary().ok())
            .ok_or_else(|| AppError::Screenshot("获取目标显示器失败".to_string()))?;

        // 尝试 WithoutBorder
        let flags = (result.clone(), captured.clone(), temp_png.clone());
        let settings = Settings::new(
            target_monitor,
            CursorCaptureSettings::WithCursor,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            flags,
        );

        let capture_handle = std::thread::spawn(move || SingleFrameCapture::start(settings));

        let first_attempt = match capture_handle.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(format!("{e}")),
            Err(_) => Err("捕获线程异常".to_string()),
        };

        // 首次失败时降级到 WithBorder
        if let Err(ref first_err) = first_attempt {
            log::debug!("WithoutBorder 失败: {first_err}，尝试 WithBorder");

            {
                let mut r = result
                    .lock()
                    .map_err(|_| AppError::Screenshot("锁错误".to_string()))?;
                r.success = false;
                r.error = None;
            }
            captured.store(false, Ordering::SeqCst);

            let flags2 = (result.clone(), captured.clone(), temp_png.clone());
            let settings2 = Settings::new(
                target_monitor,
                CursorCaptureSettings::WithCursor,
                DrawBorderSettings::WithBorder,
                SecondaryWindowSettings::Default,
                MinimumUpdateIntervalSettings::Default,
                DirtyRegionSettings::Default,
                ColorFormat::Bgra8,
                flags2,
            );

            let capture_handle2 = std::thread::spawn(move || SingleFrameCapture::start(settings2));

            match capture_handle2.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(AppError::Screenshot(format!("WithBorder 也失败: {e}"))),
                Err(_) => return Err(AppError::Screenshot("捕获线程异常".to_string())),
            }
        }

        let (success, error_msg, width, height) = {
            let r = result
                .lock()
                .map_err(|_| AppError::Screenshot("锁错误".to_string()))?;
            (r.success, r.error.clone(), r.width, r.height)
        };

        if !success {
            let msg = error_msg.unwrap_or_else(|| "未知错误".to_string());
            return Err(AppError::Screenshot(msg));
        }

        Ok((temp_png, width, height))
    }

    /// 使用 GDI BitBlt 截屏（Windows 10 兼容方案）
    #[cfg(target_os = "windows")]
    fn capture_with_gdi(
        &self,
        active_window: Option<&crate::monitor::ActiveWindow>,
    ) -> Result<(Vec<u8>, u32, u32)> {
        use std::ptr::null_mut;
        use winapi::um::wingdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
            SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
        };
        use winapi::um::winuser::{GetDC, GetSystemMetrics, ReleaseDC, SM_CXSCREEN, SM_CYSCREEN};

        unsafe {
            let (source_x, source_y, width, height) =
                capture_target_monitor_rect(active_window).unwrap_or_else(|| {
                    (
                        0,
                        0,
                        GetSystemMetrics(SM_CXSCREEN) as u32,
                        GetSystemMetrics(SM_CYSCREEN) as u32,
                    )
                });

            if width == 0 || height == 0 {
                return Err(AppError::Screenshot("获取屏幕尺寸失败".to_string()));
            }

            // 获取屏幕 DC
            let screen_dc = GetDC(null_mut());
            if screen_dc.is_null() {
                return Err(AppError::Screenshot("获取屏幕 DC 失败".to_string()));
            }

            // 创建兼容 DC
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() {
                ReleaseDC(null_mut(), screen_dc);
                return Err(AppError::Screenshot("创建兼容 DC 失败".to_string()));
            }

            // 创建兼容位图
            let bitmap = CreateCompatibleBitmap(screen_dc, width as i32, height as i32);
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(null_mut(), screen_dc);
                return Err(AppError::Screenshot("创建位图失败".to_string()));
            }

            // 选择位图到内存 DC
            let old_bitmap = SelectObject(mem_dc, bitmap as *mut _);

            // 复制屏幕内容
            let blt_result = BitBlt(
                mem_dc,
                0,
                0,
                width as i32,
                height as i32,
                screen_dc,
                source_x,
                source_y,
                SRCCOPY,
            );

            if blt_result == 0 {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(bitmap as *mut _);
                DeleteDC(mem_dc);
                ReleaseDC(null_mut(), screen_dc);
                return Err(AppError::Screenshot("BitBlt 失败".to_string()));
            }

            // 准备获取像素数据
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32), // 负值表示自上而下
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [std::mem::zeroed(); 1],
            };

            let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];

            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height,
                pixels.as_mut_ptr() as *mut _,
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // 清理资源
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut _);
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);

            if lines == 0 {
                return Err(AppError::Screenshot("GetDIBits 失败".to_string()));
            }

            // BGRA -> RGBA
            for chunk in pixels.chunks_exact_mut(4) {
                chunk.swap(0, 2); // B <-> R
            }

            log::info!(
                "GDI 截图成功: {}x{} @ ({}, {})",
                width,
                height,
                source_x,
                source_y
            );
            Ok((pixels, width, height))
        }
    }

    /// 处理临时 PNG 并保存为 JPEG
    #[cfg(target_os = "windows")]
    fn process_and_save_image(
        &self,
        temp_png: &Path,
        final_jpg: &Path,
        orig_width: u32,
        orig_height: u32,
        timestamp: i64,
    ) -> Result<ScreenshotResult> {
        let img = image::open(temp_png)
            .map_err(|e| AppError::Screenshot(format!("读取截图失败: {e}")))?;

        let mut dynamic_image = img;
        if orig_width > self.config.max_width {
            let scale = self.config.max_width as f32 / orig_width as f32;
            let new_height = (orig_height as f32 * scale) as u32;
            dynamic_image =
                dynamic_image.resize(self.config.max_width, new_height, FilterType::Lanczos3);
        }

        let final_width = dynamic_image.width();
        let final_height = dynamic_image.height();

        let rgb_image = dynamic_image.to_rgb8();
        let mut output_file = std::fs::File::create(final_jpg)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut output_file,
            self.config.jpeg_quality,
        );
        encoder.encode(
            rgb_image.as_raw(),
            final_width,
            final_height,
            ColorType::Rgb8.into(),
        )?;

        // 删除临时 PNG
        let _ = std::fs::remove_file(temp_png);

        let file_size = std::fs::metadata(final_jpg).map(|m| m.len()).unwrap_or(0);
        log::info!(
            "截屏保存到: {:?} ({}x{}, {} KB)",
            final_jpg,
            final_width,
            final_height,
            file_size / 1024
        );

        Ok(ScreenshotResult {
            path: final_jpg.to_path_buf(),
            timestamp,
            width: final_width,
            height: final_height,
        })
    }

    /// 将 RGBA 像素数据保存为 JPEG
    #[cfg(target_os = "windows")]
    fn save_rgba_to_jpeg(
        &self,
        pixels: &[u8],
        width: u32,
        height: u32,
        path: &Path,
        timestamp: i64,
    ) -> Result<ScreenshotResult> {
        // 从 RGBA 创建图像
        let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
            .ok_or_else(|| AppError::Screenshot("创建图像失败".to_string()))?;

        let mut dynamic_image = DynamicImage::ImageRgba8(img);

        // 缩放
        if width > self.config.max_width {
            let scale = self.config.max_width as f32 / width as f32;
            let new_height = (height as f32 * scale) as u32;
            dynamic_image =
                dynamic_image.resize(self.config.max_width, new_height, FilterType::Lanczos3);
        }

        let final_width = dynamic_image.width();
        let final_height = dynamic_image.height();

        // 保存为 JPEG
        let rgb_image = dynamic_image.to_rgb8();
        let mut output_file = std::fs::File::create(path)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut output_file,
            self.config.jpeg_quality,
        );
        encoder.encode(
            rgb_image.as_raw(),
            final_width,
            final_height,
            ColorType::Rgb8.into(),
        )?;

        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        log::info!(
            "GDI 截屏保存到: {:?} ({}x{}, {} KB)",
            path,
            final_width,
            final_height,
            file_size / 1024
        );

        Ok(ScreenshotResult {
            path: path.to_path_buf(),
            timestamp,
            width: final_width,
            height: final_height,
        })
    }

    /// 执行截屏（macOS）
    #[cfg(target_os = "macos")]
    fn capture_impl(
        &self,
        active_window: Option<&crate::monitor::ActiveWindow>,
    ) -> Result<ScreenshotResult> {
        use screenshots::Screen;

        let screen = if let Some((x, y)) = capture_target_point(active_window) {
            match Screen::from_point(x, y) {
                Ok(screen) => screen,
                Err(e) => {
                    log::warn!("按窗口坐标选屏失败，将回退到默认屏幕: {e}");
                    let screens = Screen::all()
                        .map_err(|err| AppError::Screenshot(format!("获取屏幕列表失败: {err}")))?;
                    screens
                        .first()
                        .copied()
                        .ok_or_else(|| AppError::Screenshot("没有找到屏幕".to_string()))?
                }
            }
        } else {
            let screens = Screen::all()
                .map_err(|e| AppError::Screenshot(format!("获取屏幕列表失败: {e}")))?;
            screens
                .first()
                .copied()
                .ok_or_else(|| AppError::Screenshot("没有找到屏幕".to_string()))?
        };

        let image = screen
            .capture()
            .map_err(|e| AppError::Screenshot(format!("截屏失败: {e}")))?;

        let orig_width = image.width();
        let orig_height = image.height();

        let mut dynamic_image = DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(orig_width, orig_height, image.into_raw())
                .ok_or_else(|| AppError::Screenshot("图像转换失败".to_string()))?,
        );

        if orig_width > self.config.max_width {
            let scale = self.config.max_width as f32 / orig_width as f32;
            let new_height = (orig_height as f32 * scale) as u32;
            dynamic_image =
                dynamic_image.resize(self.config.max_width, new_height, FilterType::Lanczos3);
        }

        let final_width = dynamic_image.width();
        let final_height = dynamic_image.height();

        let now = chrono::Local::now();
        let date_str = now.format("%Y-%m-%d").to_string();
        let time_str = now.format("%H%M%S_%3f").to_string();

        let screenshots_dir = self.data_dir.join("screenshots").join(&date_str);
        std::fs::create_dir_all(&screenshots_dir)?;

        let filename = format!("{time_str}.jpg");
        let path = screenshots_dir.join(&filename);

        let rgb_image = dynamic_image.to_rgb8();
        let mut output_file = std::fs::File::create(&path)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut output_file,
            self.config.jpeg_quality,
        );
        encoder.encode(
            rgb_image.as_raw(),
            final_width,
            final_height,
            ColorType::Rgb8.into(),
        )?;

        let timestamp = now.timestamp();
        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        log::info!(
            "截屏保存到: {:?} ({}x{}, {} KB)",
            path,
            final_width,
            final_height,
            file_size / 1024
        );

        Ok(ScreenshotResult {
            path,
            timestamp,
            width: final_width,
            height: final_height,
        })
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    fn capture_impl(&self) -> Result<ScreenshotResult> {
        Err(AppError::Screenshot("当前平台不支持截屏".to_string()))
    }

    pub fn get_relative_path(&self, full_path: &Path) -> String {
        full_path
            .strip_prefix(&self.data_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| full_path.to_string_lossy().to_string())
    }

    pub fn generate_thumbnail_base64(&self, path: &Path, max_size: u32) -> Result<String> {
        let img = image::open(path)?;
        let thumbnail = img.thumbnail(max_size, max_size);

        let rgb_thumbnail = thumbnail.to_rgb8();
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 60);
        encoder.encode(
            rgb_thumbnail.as_raw(),
            thumbnail.width(),
            thumbnail.height(),
            ColorType::Rgb8.into(),
        )?;

        Ok(BASE64_STANDARD.encode(&buffer))
    }

    pub fn calculate_image_hash(path: &Path) -> Result<u64> {
        let img = image::open(path)?;
        let small = img.resize_exact(8, 8, FilterType::Nearest).to_luma8();
        let sum: u32 = small.pixels().map(|p| p.0[0] as u32).sum();
        let avg = sum / 64;

        let mut hash: u64 = 0;
        for (i, pixel) in small.pixels().enumerate() {
            if pixel.0[0] as u32 > avg {
                hash |= 1 << i;
            }
        }

        Ok(hash)
    }

    pub fn hash_similarity(hash1: u64, hash2: u64) -> u8 {
        let xor = hash1 ^ hash2;
        let diff = xor.count_ones();
        let similarity = (64 - diff) * 100 / 64;
        similarity as u8
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn capture_target_point(
    active_window: Option<&crate::monitor::ActiveWindow>,
) -> Option<(i32, i32)> {
    let bounds = active_window?.window_bounds?;
    if bounds.width == 0 || bounds.height == 0 {
        return None;
    }

    let half_width = i32::try_from(bounds.width / 2).ok()?;
    let half_height = i32::try_from(bounds.height / 2).ok()?;
    Some((
        bounds.x.saturating_add(half_width),
        bounds.y.saturating_add(half_height),
    ))
}

#[cfg(target_os = "windows")]
fn capture_target_monitor(
    active_window: Option<&crate::monitor::ActiveWindow>,
) -> Option<windows_capture::monitor::Monitor> {
    let monitor = capture_target_hmonitor(active_window)?;
    Some(windows_capture::monitor::Monitor::from_raw_hmonitor(
        monitor as *mut std::ffi::c_void,
    ))
}

#[cfg(target_os = "windows")]
fn capture_target_monitor_rect(
    active_window: Option<&crate::monitor::ActiveWindow>,
) -> Option<(i32, i32, u32, u32)> {
    use winapi::um::winuser::{GetMonitorInfoW, MONITORINFO};

    let monitor = capture_target_hmonitor(active_window)?;
    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: unsafe { std::mem::zeroed() },
        rcWork: unsafe { std::mem::zeroed() },
        dwFlags: 0,
    };

    let ok = unsafe { GetMonitorInfoW(monitor, &mut monitor_info as *mut MONITORINFO) };
    if ok == 0 {
        return None;
    }

    let width = monitor_info
        .rcMonitor
        .right
        .checked_sub(monitor_info.rcMonitor.left)?;
    let height = monitor_info
        .rcMonitor
        .bottom
        .checked_sub(monitor_info.rcMonitor.top)?;

    if width <= 0 || height <= 0 {
        return None;
    }

    Some((
        monitor_info.rcMonitor.left,
        monitor_info.rcMonitor.top,
        width as u32,
        height as u32,
    ))
}

#[cfg(target_os = "windows")]
fn capture_target_hmonitor(
    active_window: Option<&crate::monitor::ActiveWindow>,
) -> Option<winapi::shared::windef::HMONITOR> {
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};

    let (x, y) = capture_target_point(active_window)?;
    let point = POINT { x, y };
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    if monitor.is_null() {
        None
    } else {
        Some(monitor)
    }
}

#[cfg(test)]
mod tests {
    use super::capture_target_point;
    use crate::monitor::{ActiveWindow, WindowBounds};

    #[test]
    fn 应按窗口中心点选择目标屏幕() {
        let active_window = ActiveWindow {
            app_name: "Work Review".to_string(),
            window_title: "测试窗口".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: Some(WindowBounds {
                x: 1440,
                y: 120,
                width: 1280,
                height: 800,
            }),
        };

        assert_eq!(
            capture_target_point(Some(&active_window)),
            Some((2080, 520))
        );
    }

    #[test]
    fn 缺少窗口边界时不应生成选屏坐标() {
        let active_window = ActiveWindow {
            app_name: "Work Review".to_string(),
            window_title: "测试窗口".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
        };

        assert_eq!(capture_target_point(Some(&active_window)), None);
        assert_eq!(capture_target_point(None), None);
    }
}
