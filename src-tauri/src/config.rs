use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// AI 提供商类型
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AiProvider {
    /// 本地 Ollama
    #[default]
    Ollama,
    /// OpenAI / OpenAI Compatible
    OpenAI,
    /// Google Gemini
    Gemini,
    /// Anthropic Claude
    Claude,
    /// 硅基流动 SiliconFlow
    #[serde(rename = "siliconflow")]
    SiliconFlow,
    /// DeepSeek
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// 通义千问 Qwen
    #[serde(rename = "qwen")]
    Qwen,
    /// 智谱 ChatGLM
    #[serde(rename = "zhipu")]
    Zhipu,
    /// 月之暗面 Moonshot
    #[serde(rename = "moonshot")]
    Moonshot,
    /// 火山引擎 豆包
    #[serde(rename = "doubao")]
    Doubao,
}

impl AiProvider {
    /// 获取提供商的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "Ollama (本地)",
            AiProvider::OpenAI => "OpenAI / 兼容API",
            AiProvider::Gemini => "Google Gemini",
            AiProvider::Claude => "Anthropic Claude",
            AiProvider::SiliconFlow => "硅基流动 SiliconFlow",
            AiProvider::DeepSeek => "DeepSeek",
            AiProvider::Qwen => "通义千问 Qwen",
            AiProvider::Zhipu => "智谱 ChatGLM",
            AiProvider::Moonshot => "月之暗面 Kimi",
            AiProvider::Doubao => "火山引擎 豆包",
        }
    }

    /// 获取默认的 API 地址
    pub fn default_endpoint(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "http://localhost:11434",
            AiProvider::OpenAI => "https://api.openai.com/v1",
            AiProvider::Gemini => "https://generativelanguage.googleapis.com/v1",
            AiProvider::Claude => "https://api.anthropic.com/v1",
            AiProvider::SiliconFlow => "https://api.siliconflow.cn/v1",
            AiProvider::DeepSeek => "https://api.deepseek.com",
            AiProvider::Qwen => "https://dashscope.aliyuncs.com/compatible-mode/v1",
            AiProvider::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            AiProvider::Moonshot => "https://api.moonshot.cn/v1",
            AiProvider::Doubao => "https://ark.cn-beijing.volces.com/api/v3",
        }
    }

    /// 获取默认模型名称
    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "qwen2.5",
            AiProvider::OpenAI => "gpt-4o-mini",
            AiProvider::Gemini => "gemini-1.5-flash",
            AiProvider::Claude => "claude-3-haiku-20240307",
            AiProvider::SiliconFlow => "Qwen/Qwen2.5-7B-Instruct",
            AiProvider::DeepSeek => "deepseek-chat",
            AiProvider::Qwen => "qwen-turbo",
            AiProvider::Zhipu => "glm-4-flash",
            AiProvider::Moonshot => "moonshot-v1-8k",
            AiProvider::Doubao => "doubao-lite-4k",
        }
    }

    /// 是否使用 OpenAI 兼容格式
    pub fn is_openai_compatible(&self) -> bool {
        matches!(
            self,
            AiProvider::OpenAI
                | AiProvider::SiliconFlow
                | AiProvider::DeepSeek
                | AiProvider::Qwen
                | AiProvider::Zhipu
                | AiProvider::Moonshot
                | AiProvider::Doubao
        )
    }
}

/// AI分析模式
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AiMode {
    /// 本地多模态模型（分析截图）
    #[default]
    Local,
    /// 摘要模式（只上传统计摘要）
    Summary,
    /// 云端视觉模型（上传截图到云端分析）
    Cloud,
}

/// 单个模型配置（独立的提供商、地址、密钥、模型名）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    /// 提供商类型
    pub provider: AiProvider,
    /// API 地址
    pub endpoint: String,
    /// API Key
    pub api_key: Option<String>,
    /// 模型名称
    pub model: String,
}

impl ModelConfig {
    /// 创建默认的文本模型配置
    /// 注意：model 默认为空，强制用户手动配置，避免 UI 误显示"已配置"
    pub fn default_text() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: AiProvider::Ollama.default_endpoint().to_string(),
            api_key: None,
            model: String::new(), // 默认为空，用户需手动填写
        }
    }

    /// 创建默认的视觉模型配置
    pub fn default_vision() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: AiProvider::Ollama.default_endpoint().to_string(),
            api_key: None,
            model: "llava".to_string(),
        }
    }
}

/// 可保存的文本模型档案
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextModelProfile {
    /// 档案唯一标识
    pub id: String,
    /// 档案显示名称
    pub name: String,
    /// 对应模型配置
    pub model_config: ModelConfig,
    /// 最近一次测试状态：untested / success / error
    #[serde(default = "default_connection_status")]
    pub test_status: String,
    /// 最近一次测试时间（毫秒时间戳）
    #[serde(default)]
    pub last_tested_at: Option<u64>,
    /// 最近一次测试结果描述
    #[serde(default)]
    pub last_test_message: Option<String>,
}

/// AI 提供商配置（旧版，保留用于向后兼容）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiProviderConfig {
    /// 提供商类型
    pub provider: AiProvider,
    /// API 地址（可自定义，支持代理）
    pub endpoint: String,
    /// API Key
    pub api_key: Option<String>,
    /// 模型名称
    pub model: String,
    /// 视觉模型名称（用于分析截图）
    pub vision_model: Option<String>,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: AiProvider::Ollama.default_endpoint().to_string(),
            api_key: None,
            model: AiProvider::Ollama.default_model().to_string(),
            vision_model: Some("llava".to_string()),
        }
    }
}

/// 应用隐私级别
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PrivacyLevel {
    /// 完全记录（截图 + 统计）
    #[default]
    Full,
    /// 内容脱敏（只统计时长，不保存截图）
    Anonymized,
    /// 完全忽略（不记录任何信息）
    Ignored,
}

/// 应用隐私规则
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppPrivacyRule {
    /// 应用名称
    pub app_name: String,
    /// 隐私级别（默认为 Full，兼容旧配置）
    #[serde(default)]
    pub level: PrivacyLevel,
}

/// 应用分类规则
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppCategoryRule {
    /// 应用名称
    pub app_name: String,
    /// 目标分类
    pub category: String,
}

/// 隐私配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivacyConfig {
    /// 应用隐私规则列表
    #[serde(default)]
    pub app_rules: Vec<AppPrivacyRule>,
    /// 排除的窗口标题关键词（触发时使用 Anonymized 级别）
    pub excluded_keywords: Vec<String>,
    /// URL 域名黑名单（匹配时完全忽略，不记录）
    #[serde(default)]
    pub excluded_domains: Vec<String>,
    /// 是否启用OCR敏感词过滤
    pub filter_sensitive: bool,

    // 兼容旧版
    #[serde(default)]
    pub excluded_apps: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            app_rules: vec![
                AppPrivacyRule {
                    app_name: "1Password".to_string(),
                    level: PrivacyLevel::Ignored,
                },
                AppPrivacyRule {
                    app_name: "Bitwarden".to_string(),
                    level: PrivacyLevel::Ignored,
                },
                AppPrivacyRule {
                    app_name: "Keychain".to_string(),
                    level: PrivacyLevel::Ignored,
                },
            ],
            excluded_keywords: vec![
                "bank".to_string(),
                "login".to_string(),
                "password".to_string(),
                "密码".to_string(),
                "银行".to_string(),
                "支付".to_string(),
            ],
            excluded_domains: vec![], // 默认无域名黑名单
            filter_sensitive: true,
            excluded_apps: vec![], // 旧版兼容
        }
    }
}

impl PrivacyConfig {
    /// 获取应用的隐私级别
    pub fn get_app_privacy_level(&self, app_name: &str) -> PrivacyLevel {
        let app_lower = app_name.to_lowercase();
        // 先检查新的规则（使用包含匹配，更宽容）
        for rule in &self.app_rules {
            let rule_lower = rule.app_name.to_lowercase();
            // 支持双向包含匹配：规则包含应用名 或 应用名包含规则
            if app_lower.contains(&rule_lower) || rule_lower.contains(&app_lower) {
                log::debug!(
                    "应用 {} 匹配规则 {}, 级别: {:?}",
                    app_name,
                    rule.app_name,
                    rule.level
                );
                return rule.level;
            }
        }
        // 兼容旧版 excluded_apps（视为 Ignored）
        for excluded in &self.excluded_apps {
            let excluded_lower = excluded.to_lowercase();
            if app_lower.contains(&excluded_lower) || excluded_lower.contains(&app_lower) {
                return PrivacyLevel::Ignored;
            }
        }
        PrivacyLevel::Full
    }

    /// 检查窗口标题是否触发隐私保护
    pub fn should_anonymize_by_keyword(&self, window_title: &str) -> bool {
        let title_lower = window_title.to_lowercase();
        self.excluded_keywords
            .iter()
            .any(|k| title_lower.contains(&k.to_lowercase()))
    }
}

/// 存储配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageConfig {
    /// 截图保留天数（超过后删除截图文件）
    pub screenshot_retention_days: u32,
    /// 元数据保留天数（超过后删除数据库记录）
    pub metadata_retention_days: u32,
    /// 存储空间上限（MB），超过后自动清理最旧的数据
    pub storage_limit_mb: u32,
    /// JPEG 质量 (1-100)
    pub jpeg_quality: u8,
    /// 最大图片宽度（超过会缩放）
    pub max_image_width: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            screenshot_retention_days: 7, // 默认保留7天截图
            metadata_retention_days: 30,  // 默认保留30天元数据
            storage_limit_mb: 2048,       // 默认2GB上限
            jpeg_quality: 85,             // 85%质量，更清晰
            max_image_width: 1280,        // 最大宽度1280px
        }
    }
}

/// 应用配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// 截屏间隔（秒）
    pub screenshot_interval: u64,
    /// AI分析模式
    pub ai_mode: AiMode,
    /// 文本模型配置
    #[serde(default = "ModelConfig::default_text")]
    pub text_model: ModelConfig,
    /// 可保存的文本模型档案
    #[serde(default)]
    pub text_model_profiles: Vec<TextModelProfile>,
    /// 视觉模型配置
    #[serde(default = "ModelConfig::default_vision")]
    pub vision_model: ModelConfig,
    /// 隐私配置
    pub privacy: PrivacyConfig,
    /// 应用分类覆盖规则
    #[serde(default)]
    pub app_category_rules: Vec<AppCategoryRule>,
    /// 存储配置
    #[serde(default)]
    pub storage: StorageConfig,
    /// 是否开机自启
    pub auto_start: bool,
    /// 主题模式: system, light, dark
    pub theme: String,
    /// 上班开始时间（0-23）
    #[serde(default = "default_work_start")]
    pub work_start_hour: u8,
    /// 上班结束时间（0-23）
    #[serde(default = "default_work_end")]
    pub work_end_hour: u8,
    /// 上班开始分钟（0-59）
    #[serde(default)]
    pub work_start_minute: u8,
    /// 上班结束分钟（0-59）
    #[serde(default)]
    pub work_end_minute: u8,

    // 兼容旧版配置
    #[serde(default)]
    pub ai_provider: AiProviderConfig,
    #[serde(default)]
    pub ollama_host: String,
    #[serde(default)]
    pub ollama_model: String,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default)]
    pub openai_model: String,
    /// 隐藏 Dock 图标（仅保留菜单栏）
    #[serde(default)]
    pub hide_dock_icon: bool,
    /// 轻量模式：关闭主界面时释放主 Webview，仅保留后台录制与托盘
    #[serde(default)]
    pub lightweight_mode: bool,
    /// 是否启用桌面化身窗口
    #[serde(default)]
    pub avatar_enabled: bool,
    /// 桌宠缩放比例（0.7 - 1.3）
    #[serde(default = "default_avatar_scale")]
    pub avatar_scale: f64,
    /// 桌宠猫体透明度（0.45 - 1.0）
    #[serde(default = "default_avatar_opacity")]
    pub avatar_opacity: f64,
    /// 桌宠窗口横向位置
    #[serde(default)]
    pub avatar_x: Option<i32>,
    /// 桌宠窗口纵向位置
    #[serde(default)]
    pub avatar_y: Option<i32>,
    /// 隐藏系统标题栏装饰
    #[serde(default)]
    pub hide_decorations: bool,
    /// 背景图片文件名（存储在 data 目录下）
    #[serde(default)]
    pub background_image: Option<String>,
    /// 背景图片不透明度 (0.01 - 0.60)
    #[serde(default = "default_bg_opacity")]
    pub background_opacity: f32,
    /// 背景图片模糊程度 (0 = 清晰, 1 = 轻微, 2 = 中等)
    #[serde(default = "default_bg_blur")]
    pub background_blur: u8,
}

fn default_work_start() -> u8 {
    9
}
fn default_work_end() -> u8 {
    18
}
fn default_bg_opacity() -> f32 {
    0.25
}
fn default_bg_blur() -> u8 {
    1
}
fn default_avatar_scale() -> f64 {
    0.9
}
fn default_avatar_opacity() -> f64 {
    0.82
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            screenshot_interval: 30,
            ai_mode: AiMode::Local,
            text_model: ModelConfig::default_text(),
            text_model_profiles: Vec::new(),
            vision_model: ModelConfig::default_vision(),
            privacy: PrivacyConfig::default(),
            app_category_rules: Vec::new(),
            storage: StorageConfig::default(),
            auto_start: false,
            theme: "system".to_string(),
            work_start_hour: 9,
            work_end_hour: 18,
            work_start_minute: 0,
            work_end_minute: 0,
            // 旧版兼容字段
            ai_provider: AiProviderConfig::default(),
            ollama_host: "http://localhost:11434".to_string(),
            ollama_model: "llava".to_string(),
            openai_api_key: None,
            openai_model: "gpt-4o-mini".to_string(),
            hide_dock_icon: false,
            lightweight_mode: false,
            avatar_enabled: false,
            avatar_scale: default_avatar_scale(),
            avatar_opacity: default_avatar_opacity(),
            avatar_x: None,
            avatar_y: None,
            hide_decorations: false,
            background_image: None,
            background_opacity: 0.25,
            background_blur: 1,
        }
    }
}

impl AppConfig {
    /// 规范化配置，兼容旧字段并补齐助手可用的文本模型档案
    pub fn normalize(&mut self) {
        self.migrate_legacy_config();
        normalize_app_category_rules(&mut self.app_category_rules);
        self.avatar_scale = normalize_avatar_scale(self.avatar_scale);
        self.avatar_opacity = normalize_avatar_opacity(self.avatar_opacity);
        self.sync_text_model_profiles();
    }

    /// 从文件加载配置
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let mut config: AppConfig = serde_json::from_str(&content)?;

            config.normalize();

            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置到文件
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 迁移旧版配置到新结构
    fn migrate_legacy_config(&mut self) {
        // 如果新配置为空但旧配置有值，则迁移
        if self.text_model.model.is_empty() && !self.ai_provider.model.is_empty() {
            self.text_model = ModelConfig {
                provider: self.ai_provider.provider,
                endpoint: self.ai_provider.endpoint.clone(),
                api_key: self.ai_provider.api_key.clone(),
                model: self.ai_provider.model.clone(),
            };
        }

        if self.vision_model.model.is_empty() {
            if let Some(ref vision_model) = self.ai_provider.vision_model {
                self.vision_model = ModelConfig {
                    provider: self.ai_provider.provider,
                    endpoint: self.ai_provider.endpoint.clone(),
                    api_key: self.ai_provider.api_key.clone(),
                    model: vision_model.clone(),
                };
            }
        }

        // 如果旧版 ollama 配置存在
        if !self.ollama_host.is_empty() && self.text_model.endpoint.is_empty() {
            self.text_model.endpoint = self.ollama_host.clone();
            self.vision_model.endpoint = self.ollama_host.clone();
        }

        // 迁移旧版背景不透明度（0.05 太低，用户看不到背景）
        if self.background_opacity <= 0.05 && self.background_image.is_some() {
            self.background_opacity = 0.25;
        }

        self.sync_text_model_profiles();
    }

    /// 获取文本模型端点
    pub fn get_text_endpoint(&self) -> &str {
        &self.text_model.endpoint
    }

    /// 获取视觉模型端点
    pub fn get_vision_endpoint(&self) -> &str {
        &self.vision_model.endpoint
    }

    fn sync_text_model_profiles(&mut self) {
        if !is_model_configured(&self.text_model) {
            return;
        }

        let default_profile_id = "default-text-model";
        if let Some(profile) = self
            .text_model_profiles
            .iter_mut()
            .find(|profile| profile.id == default_profile_id)
        {
            profile.name = default_profile_name(&self.text_model);
            profile.model_config = self.text_model.clone();
            if profile.test_status.trim().is_empty() {
                profile.test_status = default_connection_status();
            }
            return;
        }

        if self
            .text_model_profiles
            .iter()
            .any(|profile| same_model_config(&profile.model_config, &self.text_model))
        {
            return;
        }

        self.text_model_profiles.insert(
            0,
            TextModelProfile {
                id: default_profile_id.to_string(),
                name: default_profile_name(&self.text_model),
                model_config: self.text_model.clone(),
                test_status: default_connection_status(),
                last_tested_at: None,
                last_test_message: None,
            },
        );
    }
}

fn is_model_configured(model_config: &ModelConfig) -> bool {
    !model_config.endpoint.trim().is_empty() && !model_config.model.trim().is_empty()
}

fn same_model_config(left: &ModelConfig, right: &ModelConfig) -> bool {
    left.provider == right.provider
        && left.endpoint.trim() == right.endpoint.trim()
        && left.model.trim() == right.model.trim()
        && left.api_key.as_deref().unwrap_or("").trim()
            == right.api_key.as_deref().unwrap_or("").trim()
}

fn default_profile_name(model_config: &ModelConfig) -> String {
    format!(
        "{} · {}",
        model_config.provider.display_name(),
        model_config.model.trim()
    )
}

fn default_connection_status() -> String {
    "untested".to_string()
}

fn normalize_category_key(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "development" | "browser" | "communication" | "office" | "design" | "entertainment"
        | "other" => value.trim().to_lowercase(),
        _ => "other".to_string(),
    }
}

fn normalize_app_category_rules(rules: &mut Vec<AppCategoryRule>) {
    use std::collections::HashSet;

    let mut normalized_rules = Vec::new();
    let mut seen = HashSet::new();

    for rule in rules.iter().rev() {
        let app_name = rule.app_name.trim();
        if app_name.is_empty() {
            continue;
        }

        let normalized_app_name = crate::monitor::normalize_display_app_name(app_name);
        let dedupe_key = normalized_app_name.to_lowercase();
        if !seen.insert(dedupe_key) {
            continue;
        }

        normalized_rules.push(AppCategoryRule {
            app_name: normalized_app_name,
            category: normalize_category_key(&rule.category),
        });
    }

    normalized_rules.reverse();
    *rules = normalized_rules;
}

fn normalize_avatar_scale(value: f64) -> f64 {
    if !value.is_finite() {
        return default_avatar_scale();
    }

    value.clamp(0.7, 1.3)
}

fn normalize_avatar_opacity(value: f64) -> f64 {
    if !value.is_finite() {
        return default_avatar_opacity();
    }

    value.clamp(0.45, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{
        default_avatar_opacity, default_avatar_scale, normalize_app_category_rules,
        normalize_avatar_opacity, normalize_avatar_scale, AppCategoryRule, AppConfig,
    };

    #[test]
    fn 桌宠缩放默认值应为百分之九十() {
        let config = AppConfig::default();

        assert_eq!(config.avatar_scale, default_avatar_scale());
        assert_eq!(config.avatar_scale, 0.9);
    }

    #[test]
    fn 桌宠透明度默认值应为百分之八十二() {
        let config = AppConfig::default();

        assert_eq!(config.avatar_opacity, default_avatar_opacity());
        assert_eq!(config.avatar_opacity, 0.82);
    }

    #[test]
    fn 桌宠默认位置应为空以便首次按锚点吸附() {
        let config = AppConfig::default();

        assert_eq!(config.avatar_x, None);
        assert_eq!(config.avatar_y, None);
    }

    #[test]
    fn 轻量模式默认应关闭() {
        let config = AppConfig::default();

        assert!(!config.lightweight_mode);
    }

    #[test]
    fn 桌宠缩放应被钳制在允许范围内() {
        assert_eq!(normalize_avatar_scale(0.3), 0.7);
        assert_eq!(normalize_avatar_scale(2.0), 1.3);
        assert_eq!(normalize_avatar_scale(f64::NAN), 0.9);
    }

    #[test]
    fn 桌宠透明度应被钳制在允许范围内() {
        assert_eq!(normalize_avatar_opacity(0.1), 0.45);
        assert_eq!(normalize_avatar_opacity(1.5), 1.0);
        assert_eq!(normalize_avatar_opacity(f64::NAN), 0.82);
    }

    #[test]
    fn 应用分类规则应去重并规范化分类值() {
        let mut rules = vec![
            AppCategoryRule {
                app_name: " firefox ".to_string(),
                category: "browser".to_string(),
            },
            AppCategoryRule {
                app_name: "Firefox".to_string(),
                category: "office".to_string(),
            },
            AppCategoryRule {
                app_name: "  ".to_string(),
                category: "design".to_string(),
            },
            AppCategoryRule {
                app_name: "MuMu".to_string(),
                category: "unknown".to_string(),
            },
        ];

        normalize_app_category_rules(&mut rules);

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].app_name, "Firefox");
        assert_eq!(rules[0].category, "office");
        assert_eq!(rules[1].category, "other");
    }
}
