// OCR 文字识别模块
// Windows: 使用 PaddleOCR (轻量化版本)
// macOS: 使用 Vision 框架

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// OCR 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
    pub boxes: Vec<OcrBox>,
}

/// OCR 文字框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBox {
    pub text: String,
    pub confidence: f32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// OCR 引擎类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OcrEngine {
    /// PaddleOCR (跨平台，需要预先安装，预留功能)
    #[allow(dead_code)]
    PaddleOCR,
    /// macOS Vision 框架
    #[cfg(target_os = "macos")]
    MacOSVision,
    /// Windows OCR (通过 PowerShell)
    #[cfg(target_os = "windows")]
    WindowsOCR,
}

/// OCR 服务
pub struct OcrService {
    /// 首选引擎
    preferred_engine: OcrEngine,
    /// PaddleOCR Python 脚本路径
    paddle_script_path: Option<std::path::PathBuf>,
}

impl OcrService {
    /// 创建 OCR 服务
    pub fn new(data_dir: &Path) -> Self {
        let paddle_script_path = data_dir.join("paddle_ocr.py");

        // 检查并创建 PaddleOCR 脚本
        if !paddle_script_path.exists() {
            let _ = Self::create_paddle_script(&paddle_script_path);
        }

        // 选择默认引擎（优先使用系统原生 OCR，无需 Python 依赖）
        #[cfg(target_os = "windows")]
        let preferred_engine = OcrEngine::WindowsOCR;

        #[cfg(target_os = "macos")]
        let preferred_engine = OcrEngine::MacOSVision;

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let preferred_engine = OcrEngine::PaddleOCR;

        Self {
            preferred_engine,
            paddle_script_path: Some(paddle_script_path),
        }
    }

    /// 创建 PaddleOCR Python 脚本
    fn create_paddle_script(script_path: &Path) -> Result<()> {
        let script = r#"#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
PaddleOCR 调用脚本 (兼容 PaddleOCR 3.x)
"""

import sys
import json
import os
import warnings
warnings.filterwarnings('ignore')

# 禁用模型源检查以加速初始化
os.environ['DISABLE_MODEL_SOURCE_CHECK'] = 'True'

def run_ocr(image_path):
    try:
        from paddleocr import PaddleOCR
        
        # 新版 PaddleOCR 3.x API
        ocr = PaddleOCR(
            use_doc_orientation_classify=False,
            use_doc_unwarping=False,
            use_textline_orientation=False
        )
        
        # 使用新版 predict 方法
        result = ocr.predict(input=image_path)
        
        if result is None:
            print(json.dumps({"text": "", "boxes": [], "confidence": 0}))
            return
        
        boxes = []
        all_text = []
        total_conf = 0
        count = 0
        
        for res in result:
            # 获取识别结果
            if hasattr(res, 'rec_texts') and hasattr(res, 'rec_scores'):
                rec_boxes = getattr(res, 'dt_polys', [])
                for i, (text, conf) in enumerate(zip(res.rec_texts, res.rec_scores)):
                    box_info = {
                        "text": text,
                        "confidence": float(conf),
                        "x": 0,
                        "y": 0,
                        "width": 0,
                        "height": 0
                    }
                    # 尝试获取边界框
                    if i < len(rec_boxes):
                        box = rec_boxes[i]
                        x_coords = [p[0] for p in box]
                        y_coords = [p[1] for p in box]
                        box_info["x"] = int(min(x_coords))
                        box_info["y"] = int(min(y_coords))
                        box_info["width"] = int(max(x_coords) - box_info["x"])
                        box_info["height"] = int(max(y_coords) - box_info["y"])
                    
                    boxes.append(box_info)
                    all_text.append(text)
                    total_conf += float(conf)
                    count += 1
        
        avg_conf = total_conf / count if count > 0 else 0
        
        output = {
            "text": "\n".join(all_text),
            "boxes": boxes,
            "confidence": avg_conf
        }
        
        print(json.dumps(output, ensure_ascii=False))
        
    except ImportError:
        print(json.dumps({
            "error": "PaddleOCR not installed. Install with: pip install paddlepaddle paddleocr",
            "text": "",
            "boxes": [],
            "confidence": 0
        }))
    except Exception as e:
        print(json.dumps({
            "error": str(e),
            "text": "",
            "boxes": [],
            "confidence": 0
        }))

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No image path provided", "text": "", "boxes": [], "confidence": 0}))
        sys.exit(1)
    
    run_ocr(sys.argv[1])
"#;

        std::fs::write(script_path, script)?;
        Ok(())
    }

    /// 获取 Python 路径（优先使用 work_review conda 环境）
    fn get_python_path() -> String {
        // 优先检查 work_review conda 环境
        let conda_python = if cfg!(target_os = "windows") {
            std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
                .join("miniconda3")
                .join("envs")
                .join("work_review")
                .join("python.exe")
        } else {
            // macOS / Linux
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(&home)
                .join("all_environments")
                .join("miniconda3")
                .join("envs")
                .join("work_review")
                .join("bin")
                .join("python")
        };

        if conda_python.exists() {
            return conda_python.to_string_lossy().to_string();
        }

        // 备用：检查标准 conda 路径
        let std_conda_python = if cfg!(target_os = "windows") {
            std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
                .join("anaconda3")
                .join("envs")
                .join("work_review")
                .join("python.exe")
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(&home)
                .join("miniconda3")
                .join("envs")
                .join("work_review")
                .join("bin")
                .join("python")
        };

        if std_conda_python.exists() {
            return std_conda_python.to_string_lossy().to_string();
        }

        // 最后使用系统 python
        "python3".to_string()
    }

    /// 从图片中提取文字
    pub fn extract_text(&self, image_path: &Path) -> Result<Option<OcrResult>> {
        match self.preferred_engine {
            OcrEngine::PaddleOCR => self.extract_with_paddle(image_path),
            #[cfg(target_os = "macos")]
            OcrEngine::MacOSVision => self.extract_with_vision(image_path),
            #[cfg(target_os = "windows")]
            OcrEngine::WindowsOCR => self.extract_with_windows_ocr(image_path),
        }
    }

    /// 使用 PaddleOCR 提取文字
    fn extract_with_paddle(&self, image_path: &Path) -> Result<Option<OcrResult>> {
        let script_path = match &self.paddle_script_path {
            Some(p) => p,
            None => return Ok(None),
        };

        // 确保脚本存在
        if !script_path.exists() {
            Self::create_paddle_script(script_path)?;
        }

        // 获取 Python 路径（优先使用 work_review conda 环境）
        let python_cmd = Self::get_python_path();

        // 调用 Python 脚本
        let output = Command::new(&python_cmd)
            .arg(script_path)
            .arg(image_path)
            .output();

        match output {
            Ok(result) if result.status.success() => {
                let stdout = String::from_utf8_lossy(&result.stdout);

                // 解析 JSON 输出
                if let Ok(ocr_output) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    // 检查是否有错误
                    if let Some(error) = ocr_output.get("error") {
                        log::warn!("PaddleOCR 错误: {error}");
                        // 尝试回退到其他方法
                        #[cfg(target_os = "windows")]
                        return self.extract_with_windows_ocr(image_path);
                        #[cfg(not(target_os = "windows"))]
                        return Ok(None);
                    }

                    let text = ocr_output
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if text.is_empty() {
                        return Ok(None);
                    }

                    let confidence = ocr_output
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32;

                    let boxes: Vec<OcrBox> = ocr_output
                        .get("boxes")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();

                    log::debug!(
                        "PaddleOCR 识别到 {} 个字符, 置信度: {:.2}",
                        text.len(),
                        confidence
                    );

                    Ok(Some(OcrResult {
                        text,
                        confidence,
                        boxes,
                    }))
                } else {
                    log::warn!("无法解析 PaddleOCR 输出: {stdout}");
                    Ok(None)
                }
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                log::warn!("PaddleOCR 执行失败: {stderr}");

                // 尝试回退
                #[cfg(target_os = "windows")]
                return self.extract_with_windows_ocr(image_path);
                #[cfg(not(target_os = "windows"))]
                Ok(None)
            }
            Err(e) => {
                log::warn!("PaddleOCR 启动失败: {e}，尝试回退方案");

                // 尝试回退
                #[cfg(target_os = "windows")]
                return self.extract_with_windows_ocr(image_path);
                #[cfg(not(target_os = "windows"))]
                Ok(None)
            }
        }
    }

    /// 使用 Windows OCR API (通过 PowerShell)
    #[cfg(target_os = "windows")]
    fn extract_with_windows_ocr(&self, image_path: &Path) -> Result<Option<OcrResult>> {
        use std::os::windows::process::CommandExt;
        use std::path::PathBuf;
        use std::time::{SystemTime, UNIX_EPOCH};

        // CREATE_NO_WINDOW 标志，防止弹出黑色控制台窗口
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let powershell_path =
            PathBuf::from(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");

        let script = format!(
            r#"
$utf8 = New-Object System.Text.UTF8Encoding($false)
[Console]::OutputEncoding = $utf8
$OutputEncoding = $utf8

Add-Type -AssemblyName System.Runtime.WindowsRuntime

$imagePath = '{}'

function Write-OcrJson($payload) {{
    Write-Output (ConvertTo-Json $payload -Depth 5 -Compress)
}}

function Write-OcrError([string]$message) {{
    Write-OcrJson @{{
        text = ""
        error = ([string]$message)
        boxes = @()
        confidence = 0
    }}
}}

# 加载 Windows.Media.Ocr
[Windows.Media.Ocr.OcrEngine, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null
[Windows.Storage.StorageFile, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null
[Windows.Globalization.Language, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null
[Windows.System.UserProfile.GlobalizationPreferences, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null

# 辅助函数：等待异步操作完成
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]
if ($null -eq $asTaskGeneric) {{
    Write-OcrError "System.WindowsRuntimeSystemExtensions.AsTask 未找到"
    exit
}}

Function Await($WinRtTask, $ResultType) {{
    if ($null -eq $WinRtTask) {{
        throw "WinRT 异步任务为空: $ResultType"
    }}
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}}

try {{
    # 打开图片文件
    $file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($imagePath)) ([Windows.Storage.StorageFile])
    if ($null -eq $file) {{
        throw "读取截图文件失败: $imagePath"
    }}
    $stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
    if ($null -eq $stream) {{
        throw "打开截图流失败: $imagePath"
    }}
    
    # 解码图片
    $decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    $bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
    if ($null -eq $bitmap) {{
        throw "解码截图失败: $imagePath"
    }}
    $ocrBitmap = [Windows.Graphics.Imaging.SoftwareBitmap]::Convert(
        $bitmap,
        [Windows.Graphics.Imaging.BitmapPixelFormat]::Bgra8,
        [Windows.Graphics.Imaging.BitmapAlphaMode]::Premultiplied
    )
    if ($null -eq $ocrBitmap) {{
        throw "转换 OCR 位图失败: $imagePath"
    }}
    
    # 创建 OCR 引擎 (优先用户语言，其次简中/英文)
    $ocrEngine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
    if ($ocrEngine -eq $null) {{
        foreach ($langTag in @('zh-Hans', 'zh-Hans-CN', 'en-US')) {{
            try {{
                $language = [Windows.Globalization.Language]::new($langTag)
                $candidate = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($language)
                if ($candidate -ne $null) {{
                    $ocrEngine = $candidate
                    break
                }}
            }} catch {{
            }}
        }}
    }}
    
    if ($ocrEngine -eq $null) {{
        $profileLanguages = [string]::Join(',', [Windows.System.UserProfile.GlobalizationPreferences]::Languages)
        Write-OcrError ("No OCR engine available; user profile languages=" + ([string]$profileLanguages))
        exit
    }}
    
    # 执行 OCR
    $result = Await ($ocrEngine.RecognizeAsync($ocrBitmap)) ([Windows.Media.Ocr.OcrResult])
    if ($null -eq $result) {{
        throw "OCR 结果为空"
    }}
    
    $allText = @()
    $boxes = @()
    
    foreach ($line in $result.Lines) {{
        $allText += $line.Text
        foreach ($word in $line.Words) {{
            $rect = $word.BoundingRect
            $boxes += @{{
                text = $word.Text
                confidence = 0.9
                x = [int]$rect.X
                y = [int]$rect.Y
                width = [int]$rect.Width
                height = [int]$rect.Height
            }}
        }}
    }}
    
    $output = @{{
        text = ($allText -join "`n")
        boxes = $boxes
        confidence = 0.9
    }}
    
    Write-OcrJson $output
    
    try {{ $stream.Dispose() }} catch {{}}
    try {{ $bitmap.Dispose() }} catch {{}}
    try {{ $ocrBitmap.Dispose() }} catch {{}}
}} catch {{
    $message = if ($_.Exception -and $_.Exception.Message) {{
        [string]$_.Exception.Message
    }} else {{
        [string]$_
    }}
    Write-OcrError $message
}}
"#,
            image_path
                .to_string_lossy()
                .replace("'", "''")
        );

        let script_name = format!(
            "work_review_ocr_{}.ps1",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let script_path = std::env::temp_dir().join(script_name);

        // 写入时添加 UTF-8 BOM，确保 Windows PowerShell 正确识别编码
        let bom: &[u8] = b"\xEF\xBB\xBF";
        let script_bytes = script.as_bytes();
        let mut content = Vec::with_capacity(bom.len() + script_bytes.len());
        content.extend_from_slice(bom);
        content.extend_from_slice(script_bytes);

        if let Err(e) = std::fs::write(&script_path, &content) {
            log::warn!("写入 Windows OCR 临时脚本失败: {e}");
            return Ok(None);
        }

        let output = Command::new(&powershell_path)
            .args([
                "-NoProfile",
                "-Sta",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script_path.to_string_lossy().as_ref(),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let _ = std::fs::remove_file(&script_path);

        match output {
            Ok(result) if result.status.success() => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                if let Ok(ocr_output) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(error) = ocr_output.get("error").and_then(|v| v.as_str()) {
                        log::warn!("Windows OCR 错误: {error}");
                        if !stderr.trim().is_empty() {
                            log::warn!("Windows OCR stderr: {}", stderr.trim());
                        }
                        return Ok(None);
                    }

                    let text = ocr_output
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if text.is_empty() {
                        return Ok(None);
                    }

                    let confidence = ocr_output
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.9) as f32;

                    let boxes: Vec<OcrBox> = ocr_output
                        .get("boxes")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default();

                    log::debug!("Windows OCR 识别到 {} 个字符", text.len());

                    Ok(Some(OcrResult {
                        text,
                        confidence,
                        boxes,
                    }))
                } else {
                    if !stdout.trim().is_empty() {
                        log::warn!("Windows OCR 输出无法解析为 JSON: {}", stdout.trim());
                    }
                    if !stderr.trim().is_empty() {
                        log::warn!("Windows OCR stderr: {}", stderr.trim());
                    }
                    Ok(None)
                }
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                let stdout = String::from_utf8_lossy(&result.stdout);
                log::warn!(
                    "Windows OCR PowerShell 执行失败: status={:?}, stderr={}, stdout={}",
                    result.status.code(),
                    stderr.trim(),
                    stdout.trim()
                );
                Ok(None)
            }
            Err(e) => {
                log::warn!("Windows OCR PowerShell 启动失败: {e}");
                Ok(None)
            }
        }
    }

    /// 使用 macOS Vision 框架提取文字
    #[cfg(target_os = "macos")]
    fn extract_with_vision(&self, image_path: &Path) -> Result<Option<OcrResult>> {
        let script = format!(
            r#"
use framework "Vision"
use framework "Foundation"
use scripting additions

set imagePath to "{}"
set theImage to current application's NSImage's alloc()'s initWithContentsOfFile:imagePath

if theImage is missing value then
    return ""
end if

set requestHandler to current application's VNImageRequestHandler's alloc()'s initWithData:(theImage's TIFFRepresentation()) options:(current application's NSDictionary's dictionary())
set theRequest to current application's VNRecognizeTextRequest's alloc()'s init()
theRequest's setRecognitionLevel:(current application's VNRequestTextRecognitionLevelAccurate)
theRequest's setRecognitionLanguages:{{"zh-Hans", "en"}}

requestHandler's performRequests:{{theRequest}} |error|:(missing value)

set theResults to theRequest's results()
set outputText to ""

repeat with observation in theResults
    set outputText to outputText & (observation's topCandidates:1)'s firstObject()'s |string|() & linefeed
end repeat

return outputText
            "#,
            image_path.to_string_lossy()
        );

        let output = Command::new("osascript")
            .arg("-l")
            .arg("AppleScript")
            .arg("-e")
            .arg(&script)
            .output();

        match output {
            Ok(result) if result.status.success() => {
                let text = String::from_utf8_lossy(&result.stdout).trim().to_string();
                if text.is_empty() {
                    Ok(None)
                } else {
                    log::debug!("Vision OCR 识别到 {} 个字符", text.len());
                    Ok(Some(OcrResult {
                        text,
                        confidence: 0.9,
                        boxes: vec![],
                    }))
                }
            }
            Ok(result) => {
                log::warn!(
                    "Vision OCR 命令执行失败: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
                Ok(None)
            }
            Err(e) => {
                log::warn!("Vision OCR 执行错误: {e}");
                Ok(None)
            }
        }
    }

    /// 检查 PaddleOCR 是否可用
    pub fn check_paddle_available() -> bool {
        let python_cmd = Self::get_python_path();
        let output = Command::new(&python_cmd)
            .args(["-c", "import paddleocr; print('ok')"])
            .output();

        match output {
            Ok(result) => {
                result.status.success() && String::from_utf8_lossy(&result.stdout).trim() == "ok"
            }
            Err(_) => false,
        }
    }

    /// 获取安装 PaddleOCR 的命令
    pub fn get_paddle_install_command() -> &'static str {
        "pip install paddlepaddle paddleocr -i https://mirror.baidu.com/pypi/simple"
    }
}

/// 清理 OCR 文本（剔除乱码、Markdown 符号等）
pub fn clean_ocr_text(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in text.lines() {
        let cleaned = clean_line(line);

        // 跳过空行或太短的行（可能是乱码）
        if cleaned.len() < 2 {
            continue;
        }

        // 跳过重复行
        let normalized = cleaned.to_lowercase();
        if seen.contains(&normalized) {
            continue;
        }
        seen.insert(normalized);

        lines.push(cleaned);
    }

    lines.join("\n")
}

/// 清理单行文本
fn clean_line(line: &str) -> String {
    let mut result = String::new();

    for c in line.chars() {
        // 只保留：中文、英文、数字、常用标点
        if is_valid_char(c) {
            result.push(c);
        } else if c.is_whitespace() {
            // 空格保留，但连续空格合并
            if !result.ends_with(' ') && !result.is_empty() {
                result.push(' ');
            }
        }
    }

    result.trim().to_string()
}

/// 判断字符是否有效（中英文、数字、常用标点）
fn is_valid_char(c: char) -> bool {
    // 中文字符
    if ('\u{4e00}'..='\u{9fff}').contains(&c) {
        return true;
    }
    // 英文字母
    if c.is_ascii_alphabetic() {
        return true;
    }
    // 数字
    if c.is_ascii_digit() {
        return true;
    }
    // 常用标点（中英文）
    let punctuation: [char; 30] = [
        '，', '。', '！', '？', '、', '；', '：', '\u{201c}', '\u{201d}', '\u{2018}', '\u{2019}',
        '（', '）', '【', '】', '「', '」', '《', '》', '-', '—', '·', '.', ',', ':', ';', '!',
        '?', '(', ')',
    ];
    if punctuation.contains(&c) {
        return true;
    }

    false
}

/// 过滤敏感信息
pub fn filter_sensitive_text(text: &str) -> String {
    use regex::Regex;

    // 先清理乱码和特殊字符
    let cleaned = clean_ocr_text(text);
    let mut result = cleaned;

    // 过滤手机号
    if let Ok(re) = Regex::new(r"1[3-9]\d{9}") {
        result = re.replace_all(&result, "[手机号]").to_string();
    }

    // 过滤身份证号
    if let Ok(re) = Regex::new(r"\d{17}[\dXx]") {
        result = re.replace_all(&result, "[身份证号]").to_string();
    }

    // 过滤银行卡号（要求独立出现的 16-19 位数字，或 4 位一组以空格/横线分隔）
    if let Ok(re) = Regex::new(r"(?<!\d)\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{0,3}(?!\d)")
    {
        result = re.replace_all(&result, "[银行卡号]").to_string();
    }

    // 过滤邮箱
    if let Ok(re) = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}") {
        result = re.replace_all(&result, "[邮箱]").to_string();
    }

    // 过滤 IP 地址
    if let Ok(re) = Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b") {
        result = re.replace_all(&result, "[IP地址]").to_string();
    }

    // 过滤密码相关
    if let Ok(re) = Regex::new(r"(?i)(password|密码|pwd)[:\s=]*\S+") {
        result = re.replace_all(&result, "[密码信息]").to_string();
    }

    result
}
