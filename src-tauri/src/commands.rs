use crate::config::{AiProvider, AiProviderConfig, AppCategoryRule, AppConfig, ModelConfig};
use crate::database::Database;
use crate::database::{Activity, DailyReport, DailyStats, MemorySearchItem};
use crate::error::AppError;
use crate::privacy::PrivacyFilter;
use crate::screenshot::ScreenshotService;
use crate::storage::StorageManager;
use crate::work_intelligence::{
    analyze_intents, build_work_sessions, extract_todos,
    generate_weekly_review as build_weekly_review, IntentAnalysisResult, TodoExtractionResult,
    WeeklyReviewResult, WorkSession,
};
use crate::AppState;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/wm94i/Work_Review/releases/latest";
const GITHUB_LATEST_RELEASE_PAGE: &str = "https://github.com/wm94i/Work_Review/releases/latest";
const UPDATE_STATUS_EVENT: &str = "update-status";
const UPDATER_JSON_ENDPOINTS: &[&str] = &[
    "https://ghproxy.cn/https://github.com/wm94i/Work_Review/releases/latest/download/updater-ghproxy.json",
    "https://ghp.ci/https://github.com/wm94i/Work_Review/releases/latest/download/updater-ghp.json",
    "https://github.com/wm94i/Work_Review/releases/latest/download/updater.json",
];
const DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: u64 = 24;
const MANAGED_DATA_ENTRIES: &[&str] = &[
    "config.json",
    "workreview.db",
    "screenshots",
    "ocr_logs",
    "background.jpg",
    "update_settings.json",
];

/// 模型测试结果
#[derive(Serialize, Deserialize, Debug)]
pub struct ModelTestResult {
    pub success: bool,
    pub message: String,
    pub response_time_ms: u64,
    pub model_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppCategoryOverviewItem {
    pub app_name: String,
    pub category: String,
    pub total_duration: i64,
    pub is_overridden: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAnswer {
    pub answer: String,
    pub references: Vec<MemorySearchItem>,
    pub used_ai: bool,
    pub model_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssistantAnswer {
    pub answer: String,
    pub references: Vec<MemorySearchItem>,
    pub used_ai: bool,
    pub model_name: Option<String>,
    pub tool_labels: Vec<String>,
    pub cards: Vec<AssistantCard>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssistantChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssistantCard {
    pub kind: String,
    pub title: String,
    pub content: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GithubUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub auto_update_ready: bool,
    pub release_url: String,
    pub body: Option<String>,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GithubUpdateInstallResult {
    pub updated: bool,
    pub available: bool,
    pub version: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub attempted_sources: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GithubUpdateStatusPayload {
    stage: String,
    message: String,
    source: Option<String>,
    version: Option<String>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    percent: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub last_check_time: u64,
    #[serde(default = "default_update_check_interval")]
    pub check_interval_hours: u64,
}

#[derive(Deserialize, Debug)]
struct GithubReleaseResponse {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct UpdaterJsonResponse {
    version: String,
    notes: Option<String>,
    #[allow(dead_code)]
    pub_date: Option<String>,
}

fn default_update_check_interval() -> u64 {
    DEFAULT_UPDATE_CHECK_INTERVAL_HOURS
}

fn update_source_label(endpoint: &str) -> String {
    Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_string()))
        .unwrap_or_else(|| endpoint.to_string())
}

fn build_versioned_updater_endpoint(endpoint: &str, version: &str) -> Option<String> {
    let normalized_version = normalize_version(version);
    if normalized_version.is_empty() {
        return None;
    }

    endpoint.contains("releases/latest/download/").then(|| {
        endpoint.replacen(
            "releases/latest/download/",
            &format!("releases/download/v{normalized_version}/"),
            1,
        )
    })
}

fn build_updater_manifest_candidates(
    endpoint: &str,
    expected_version: Option<&str>,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(expected_version) = expected_version {
        if let Some(versioned_endpoint) =
            build_versioned_updater_endpoint(endpoint, expected_version)
        {
            candidates.push(versioned_endpoint);
        }
    }

    candidates.push(endpoint.to_string());
    candidates.dedup();
    candidates
}

fn emit_update_status(
    app: &AppHandle,
    stage: &str,
    message: impl Into<String>,
    source: Option<String>,
    version: Option<String>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    percent: Option<u64>,
) {
    let _ = app.emit(
        UPDATE_STATUS_EVENT,
        GithubUpdateStatusPayload {
            stage: stage.to_string(),
            message: message.into(),
            source,
            version,
            downloaded_bytes,
            total_bytes,
            percent,
        },
    );
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            last_check_time: 0,
            check_interval_hours: DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
        }
    }
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches(['v', 'V'])
}

fn parse_version_parts(version: &str) -> Vec<u64> {
    normalize_version(version)
        .split('.')
        .map(|segment| {
            segment
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

fn compare_versions(current: &str, latest: &str) -> Ordering {
    let current_parts = parse_version_parts(current);
    let latest_parts = parse_version_parts(latest);
    let max_len = current_parts.len().max(latest_parts.len());

    for index in 0..max_len {
        let current_value = *current_parts.get(index).unwrap_or(&0);
        let latest_value = *latest_parts.get(index).unwrap_or(&0);
        match current_value.cmp(&latest_value) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    Ordering::Equal
}

fn is_text_model_available(model_config: &ModelConfig) -> bool {
    !model_config.endpoint.trim().is_empty() && !model_config.model.trim().is_empty()
}

/// 从问题中提取时间范围关键词，返回 (date_from, date_to)
fn parse_temporal_range(question: &str) -> (Option<String>, Option<String>) {
    use chrono::{Datelike, Local};

    let normalized = question.trim().to_lowercase();
    let today = Local::now().date_naive();
    let fmt = |d: chrono::NaiveDate| d.format("%Y-%m-%d").to_string();

    // 今天/今日
    if normalized.contains("今天") || normalized.contains("今日") {
        let d = fmt(today);
        return (Some(d.clone()), Some(d));
    }

    // 昨天/昨日
    if normalized.contains("昨天") || normalized.contains("昨日") {
        let d = fmt(today - chrono::Duration::days(1));
        return (Some(d.clone()), Some(d));
    }

    // 前天
    if normalized.contains("前天") {
        let d = fmt(today - chrono::Duration::days(2));
        return (Some(d.clone()), Some(d));
    }

    // 最近N天/近N天/过去N天 — 用 regex 提取数字
    if let Ok(re) = regex::Regex::new(r"(?:最近|近|过去)\s*(\d+)\s*天") {
        if let Some(caps) = re.captures(&normalized) {
            if let Ok(n) = caps[1].parse::<i64>() {
                return (
                    Some(fmt(today - chrono::Duration::days(n))),
                    Some(fmt(today)),
                );
            }
        }
    }

    // 含"最近"但无数字 → 默认 7 天
    if normalized.contains("最近") {
        return (
            Some(fmt(today - chrono::Duration::days(7))),
            Some(fmt(today)),
        );
    }

    // 本周/这周
    if normalized.contains("本周") || normalized.contains("这周") {
        let wd = today.weekday().num_days_from_monday() as i64;
        let monday = today - chrono::Duration::days(wd);
        return (Some(fmt(monday)), Some(fmt(today)));
    }

    // 上周/上一周
    if normalized.contains("上周") || normalized.contains("上一周") {
        let wd = today.weekday().num_days_from_monday() as i64;
        let this_monday = today - chrono::Duration::days(wd);
        let last_monday = this_monday - chrono::Duration::days(7);
        let last_sunday = this_monday - chrono::Duration::days(1);
        return (Some(fmt(last_monday)), Some(fmt(last_sunday)));
    }

    // 本月/这个月
    if normalized.contains("本月") || normalized.contains("这个月") {
        let first = today.with_day(1).unwrap_or(today);
        return (Some(fmt(first)), Some(fmt(today)));
    }

    // 上月/上个月
    if normalized.contains("上月") || normalized.contains("上个月") {
        let first_this = today.with_day(1).unwrap_or(today);
        let last_day_prev = first_this - chrono::Duration::days(1);
        let first_prev = last_day_prev.with_day(1).unwrap_or(last_day_prev);
        return (Some(fmt(first_prev)), Some(fmt(last_day_prev)));
    }

    (None, None)
}

fn format_memory_references(references: &[MemorySearchItem]) -> String {
    references
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let source_label = match item.source_type.as_str() {
                "activity" => "活动记录",
                "hourly_summary" => "小时摘要",
                "daily_report" => "日报",
                _ => "记忆",
            };

            let mut parts = vec![format!("{}. [{}] {}", index + 1, source_label, item.title)];
            parts.push(format!("日期: {}", item.date));

            if let Some(app_name) = &item.app_name {
                if !app_name.is_empty() {
                    parts.push(format!("应用: {app_name}"));
                }
            }

            if let Some(browser_url) = &item.browser_url {
                if !browser_url.is_empty() {
                    parts.push(format!(
                        "URL: {}",
                        format_browser_url_for_display(browser_url)
                    ));
                }
            }

            if let Some(duration) = item.duration {
                if duration > 0 {
                    parts.push(format!("时长: {}秒", duration));
                }
            }

            if !item.excerpt.is_empty() {
                parts.push(format!("内容: {}", item.excerpt));
            }

            parts.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_browser_url_for_display(raw_url: &str) -> String {
    let mut output = String::with_capacity(raw_url.len());
    let bytes = raw_url.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            output.push(bytes[index] as char);
            index += 1;
            continue;
        }

        let start = index;
        let mut decoded_bytes = Vec::new();

        while index + 2 < bytes.len() && bytes[index] == b'%' {
            let hex = &raw_url[index + 1..index + 3];
            let Ok(value) = u8::from_str_radix(hex, 16) else {
                break;
            };
            decoded_bytes.push(value);
            index += 3;
        }

        if decoded_bytes.is_empty() {
            output.push('%');
            index = start + 1;
            continue;
        }

        let raw_segment = &raw_url[start..index];
        if !decoded_bytes.iter().any(|byte| *byte >= 0x80) {
            output.push_str(raw_segment);
            continue;
        }

        match String::from_utf8(decoded_bytes) {
            Ok(decoded) => output.push_str(&decoded),
            Err(_) => output.push_str(raw_segment),
        }
    }

    output
}

fn is_low_signal_reference(item: &MemorySearchItem) -> bool {
    let title = item.title.trim().to_lowercase();
    let excerpt = item.excerpt.trim().to_lowercase();

    let menu_terms = [
        "文件",
        "编辑",
        "显示",
        "窗口",
        "帮助",
        "历史记录",
        "书签",
        "标签页",
        "个人资料",
        "记录状态",
        "时间线",
        "日报",
        "window",
        "help",
        "file",
        "edit",
        "view",
    ];

    let menu_hits = menu_terms
        .iter()
        .filter(|term| excerpt.contains(**term))
        .count();
    let path_like_title = title.contains('/') || title.contains('\\');
    let generic_title = title.starts_with("无标题")
        || title.starts_with("untitled")
        || title == "work review"
        || title.starts_with("work review -");
    let browser_shell_title =
        title.contains("google chrome") && item.browser_url.as_deref().unwrap_or("").is_empty();

    menu_hits >= 5 || ((path_like_title || generic_title || browser_shell_title) && menu_hits >= 3)
}

fn filter_reference_items<'a>(
    references: &'a [MemorySearchItem],
    limit: usize,
) -> Vec<&'a MemorySearchItem> {
    references
        .iter()
        .filter(|item| !is_low_signal_reference(item))
        .take(limit)
        .collect()
}

fn build_memory_answer_prompt(question: &str, references: &[MemorySearchItem]) -> String {
    format!(
        "你是一个个人工作记忆助手。请严格基于给定记录回答，不要编造未出现的事实。\n\
如果证据不足，要明确说“不确定”或“记录里没有显示”。\n\
优先回答时间、应用、网站、工作主题和依据。\n\
回答请用中文，结构简洁，可使用短段落或要点。\n\n\
用户问题：{question}\n\n\
相关记录：\n{refs}",
        refs = format_memory_references(references)
    )
}

fn build_fallback_memory_answer(question: &str, references: &[MemorySearchItem]) -> String {
    if references.is_empty() {
        return format!(
            "未找到和“{question}”相关的历史记录。\n\n可尝试换一个关键词，或缩小日期范围后再搜索。"
        );
    }

    let mut answer = String::new();
    answer.push_str("以下是检索到的相关记录。\n\n");

    for item in references.iter().take(5) {
        answer.push_str(&format!("- {}（{}）", item.title, item.date));
        if let Some(app_name) = &item.app_name {
            if !app_name.is_empty() {
                answer.push_str(&format!("，应用：{app_name}"));
            }
        }
        if let Some(browser_url) = &item.browser_url {
            if !browser_url.is_empty() {
                answer.push_str(&format!(
                    "，URL：{}",
                    format_browser_url_for_display(browser_url)
                ));
            }
        }
        if let Some(duration) = item.duration {
            if duration > 0 {
                answer.push_str(&format!("，时长约 {} 秒", duration));
            }
        }
        if !item.excerpt.is_empty() {
            answer.push_str(&format!("。摘要：{}", item.excerpt));
        }
        answer.push('\n');
    }

    answer.push_str("\n当前为基础回答模式，仅基于检索结果做整理，未启用大模型归纳。");
    answer
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssistantTool {
    Memory,
    Sessions,
    Intents,
    Review,
    Todos,
}

impl AssistantTool {
    fn label(&self) -> &'static str {
        match self {
            AssistantTool::Memory => "记忆检索",
            AssistantTool::Sessions => "Session 聚合",
            AssistantTool::Intents => "意图识别",
            AssistantTool::Review => "周报复盘",
            AssistantTool::Todos => "待办提取",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssistantQuestionKind {
    StageSummary,
    OutcomeRecap,
    ProcessRecap,
    EvidenceQuery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssistantReasoningMode {
    Basic,
    AiEnhanced,
}

impl AssistantQuestionKind {
    fn label(&self) -> &'static str {
        match self {
            AssistantQuestionKind::StageSummary => "阶段总结",
            AssistantQuestionKind::OutcomeRecap => "结果复盘",
            AssistantQuestionKind::ProcessRecap => "过程复盘",
            AssistantQuestionKind::EvidenceQuery => "依据追问",
        }
    }
}

fn build_history_context(history: &[AssistantChatMessage]) -> String {
    history
        .iter()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|message| format!("{}: {}", message.role, message.content.trim()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_short_follow_up_question(question: &str) -> bool {
    let trimmed = question.trim();
    let normalized = trimmed.to_lowercase();

    trimmed.chars().count() <= 18
        && [
            "继续",
            "展开",
            "细说",
            "详细",
            "具体",
            "接着",
            "那",
            "这个",
            "这里",
            "这个结论",
            "说说",
            "依据",
        ]
        .iter()
        .any(|pattern| normalized.contains(pattern))
}

fn assistant_reasoning_mode(model_config: Option<&ModelConfig>) -> AssistantReasoningMode {
    if model_config.is_some_and(is_text_model_available) {
        AssistantReasoningMode::AiEnhanced
    } else {
        AssistantReasoningMode::Basic
    }
}

fn build_contextual_query(question: &str, history: &[AssistantChatMessage]) -> String {
    let trimmed = question.trim();
    if trimmed.chars().count() >= 8 && !is_short_follow_up_question(trimmed) {
        return trimmed.to_string();
    }

    let previous_user = history
        .iter()
        .rev()
        .find(|message| message.role == "user" && message.content.trim() != trimmed)
        .map(|message| message.content.trim().to_string());

    if let Some(previous_user) = previous_user {
        format!("{previous_user} {trimmed}")
    } else {
        trimmed.to_string()
    }
}

fn build_question_analysis_context(question: &str, history: &[AssistantChatMessage]) -> String {
    let trimmed = question.trim();
    if history.is_empty() {
        return trimmed.to_lowercase();
    }

    let should_expand = trimmed.chars().count() <= 18
        || [
            "这个",
            "这个结论",
            "这里",
            "这些",
            "它",
            "上面",
            "刚才",
            "继续",
            "展开",
            "依据",
        ]
        .iter()
        .any(|pattern| trimmed.contains(pattern));

    if !should_expand {
        return trimmed.to_lowercase();
    }

    let mut context = build_history_context(history);
    if !context.is_empty() {
        context.push('\n');
    }
    context.push_str(trimmed);
    context.to_lowercase()
}

fn detect_question_kind_from_text(text: &str) -> AssistantQuestionKind {
    let context = text.trim().to_lowercase();

    if context.is_empty() {
        return AssistantQuestionKind::StageSummary;
    }

    let evidence_patterns = [
        "依据",
        "证据",
        "怎么得出",
        "怎么判断",
        "为什么这么说",
        "哪些记录",
        "哪条记录",
        "从哪里看",
        "原文",
    ];
    if evidence_patterns
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        return AssistantQuestionKind::EvidenceQuery;
    }

    let process_patterns = [
        "过程",
        "怎么推进",
        "时间花在哪",
        "花在哪",
        "节奏",
        "session",
        "工作段",
        "时段",
        "时间线",
        "切换",
        "过程复盘",
    ];
    if process_patterns
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        return AssistantQuestionKind::ProcessRecap;
    }

    let outcome_patterns = [
        "结果",
        "产出",
        "完成了什么",
        "推进到哪",
        "进展",
        "交付",
        "没收口",
        "待办",
        "下一步",
        "后续",
        "风险",
        "阻塞",
    ];
    if outcome_patterns
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        return AssistantQuestionKind::OutcomeRecap;
    }

    AssistantQuestionKind::StageSummary
}

fn last_user_question_kind(history: &[AssistantChatMessage]) -> Option<AssistantQuestionKind> {
    history
        .iter()
        .rev()
        .find(|message| message.role == "user" && !message.content.trim().is_empty())
        .map(|message| detect_question_kind_from_text(&message.content))
}

fn infer_question_kind_from_assistant_reply(
    history: &[AssistantChatMessage],
) -> Option<AssistantQuestionKind> {
    let content = history
        .iter()
        .rev()
        .find(|message| message.role == "assistant" && !message.content.trim().is_empty())
        .map(|message| message.content.trim().to_lowercase())?;

    let mut best_kind = AssistantQuestionKind::StageSummary;
    let mut best_score = 0i32;

    let candidates: [(AssistantQuestionKind, &[&str]); 4] = [
        (
            AssistantQuestionKind::EvidenceQuery,
            &[
                "## 依据补充",
                "依据",
                "记录",
                "原始记录",
                "证据",
                "哪条记录",
            ],
        ),
        (
            AssistantQuestionKind::ProcessRecap,
            &[
                "## 过程分析",
                "session",
                "工作段",
                "时间花在",
                "推进片段",
                "切换",
            ],
        ),
        (
            AssistantQuestionKind::OutcomeRecap,
            &["待办", "风险", "交付", "结果概览", "收口", "下一步"],
        ),
        (
            AssistantQuestionKind::StageSummary,
            &["结论", "主线", "阶段", "主要做了什么", "工作重心"],
        ),
    ];

    for (kind, patterns) in candidates {
        let score = patterns
            .iter()
            .map(|pattern| {
                if content.contains(pattern) {
                    if pattern.starts_with("## ") {
                        3
                    } else {
                        1
                    }
                } else {
                    0
                }
            })
            .sum::<i32>();

        if score > best_score {
            best_score = score;
            best_kind = kind;
        }
    }

    if best_score > 0 {
        Some(best_kind)
    } else {
        None
    }
}

fn detect_assistant_question_kind_with_mode(
    question: &str,
    history: &[AssistantChatMessage],
    mode: AssistantReasoningMode,
) -> AssistantQuestionKind {
    let trimmed = question.trim();
    let current_kind = detect_question_kind_from_text(trimmed);

    if current_kind == AssistantQuestionKind::EvidenceQuery {
        return current_kind;
    }

    if is_short_follow_up_question(trimmed) {
        if mode == AssistantReasoningMode::AiEnhanced {
            if let Some(assistant_kind) = infer_question_kind_from_assistant_reply(history) {
                if assistant_kind != AssistantQuestionKind::StageSummary {
                    return assistant_kind;
                }
            }
        }

        if let Some(previous_kind) = last_user_question_kind(history) {
            return previous_kind;
        }
    }

    let context = build_question_analysis_context(question, history);
    let contextual_kind = detect_question_kind_from_text(&context);
    if contextual_kind != AssistantQuestionKind::StageSummary {
        return contextual_kind;
    }

    current_kind
}

fn detect_assistant_question_kind(
    question: &str,
    history: &[AssistantChatMessage],
) -> AssistantQuestionKind {
    detect_assistant_question_kind_with_mode(question, history, AssistantReasoningMode::Basic)
}

fn unique_assistant_tools(tools: Vec<AssistantTool>) -> Vec<AssistantTool> {
    let mut unique = Vec::new();
    for tool in tools {
        if !unique.contains(&tool) {
            unique.push(tool);
        }
    }
    unique
}

fn map_question_kind_to_tools(kind: AssistantQuestionKind) -> Vec<AssistantTool> {
    match kind {
        AssistantQuestionKind::StageSummary => {
            vec![
                AssistantTool::Review,
                AssistantTool::Intents,
                AssistantTool::Memory,
            ]
        }
        AssistantQuestionKind::OutcomeRecap => vec![
            AssistantTool::Review,
            AssistantTool::Intents,
            AssistantTool::Todos,
            AssistantTool::Memory,
        ],
        AssistantQuestionKind::ProcessRecap => {
            vec![
                AssistantTool::Sessions,
                AssistantTool::Intents,
                AssistantTool::Memory,
            ]
        }
        AssistantQuestionKind::EvidenceQuery => vec![
            AssistantTool::Memory,
            AssistantTool::Sessions,
            AssistantTool::Intents,
        ],
    }
}

fn detect_assistant_tools_with_history(
    question: &str,
    history: &[AssistantChatMessage],
    mode: AssistantReasoningMode,
) -> Vec<AssistantTool> {
    let kind = detect_assistant_question_kind_with_mode(question, history, mode);
    let context = build_question_analysis_context(question, history);
    let mut tools = map_question_kind_to_tools(kind);

    if ["session", "工作段", "时段", "切换"]
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        tools.push(AssistantTool::Sessions);
    }

    if ["待办", "todo", "后续", "下一步", "没收口"]
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        tools.push(AssistantTool::Todos);
    }

    if ["复盘", "总结", "回顾", "这周", "本周", "上周"]
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        tools.push(AssistantTool::Review);
    }

    if ["重心", "方向", "主要工作", "主要做了什么"]
        .iter()
        .any(|pattern| context.contains(pattern))
    {
        tools.push(AssistantTool::Intents);
    }

    tools.push(AssistantTool::Memory);
    unique_assistant_tools(tools)
}

fn summarize_sessions_for_prompt(sessions: &[WorkSession]) -> String {
    if sessions.is_empty() {
        return "无 session 数据".to_string();
    }

    sessions
        .iter()
        .take(6)
        .enumerate()
        .map(|(index, session)| {
            format!(
                "{}. {} {}-{}，{}，主应用：{}，意图：{}，标题：{}",
                index + 1,
                session.date,
                session.start_timestamp,
                session.end_timestamp,
                crate::analysis::format_duration(session.duration),
                session.dominant_app,
                session.intent_label,
                session.title
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_intents_for_prompt(result: &IntentAnalysisResult) -> String {
    if result.summary.is_empty() {
        return "无意图识别结果".to_string();
    }

    result
        .summary
        .iter()
        .take(6)
        .map(|item| {
            format!(
                "- {}：{}，{} 段",
                item.label,
                crate::analysis::format_duration(item.duration),
                item.session_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn summarize_todos_for_prompt(result: &TodoExtractionResult) -> String {
    if result.items.is_empty() {
        return "无待办提取结果".to_string();
    }

    result
        .items
        .iter()
        .take(8)
        .map(|item| {
            format!(
                "- {}（{}，{}，置信度 {}，{}）",
                item.title, item.date, item.source_app, item.confidence, item.reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_assistant_prompt(
    question: &str,
    question_kind: AssistantQuestionKind,
    history: &[AssistantChatMessage],
    date_from: Option<&str>,
    date_to: Option<&str>,
    references: &[MemorySearchItem],
    sessions: Option<&[WorkSession]>,
    intents: Option<&IntentAnalysisResult>,
    review: Option<&WeeklyReviewResult>,
    todos: Option<&TodoExtractionResult>,
) -> String {
    let range = match (date_from, date_to) {
        (Some(start), Some(end)) if start == end => format!("{start} 当天"),
        (Some(start), Some(end)) => format!("{start} 到 {end}"),
        (Some(start), None) => format!("{start} 之后"),
        (None, Some(end)) => format!("{end} 之前"),
        (None, None) => "全部可用记录".to_string(),
    };

    let mut prompt = format!(
        "用户问题：{question}\n问题类型：{}\n数据时间范围：{range}（以下所有数据均在此范围内，超出范围的信息不可用）\n\n请用“分析复盘型”风格直接回答。严格基于以下数据，不要编造未出现的事实；证据不足时明确说明。\n",
        question_kind.label()
    );

    let recent_history: Vec<_> = history.iter().rev().take(4).collect::<Vec<_>>();
    if !recent_history.is_empty() {
        prompt.push_str("\n【对话上下文】\n");
        for msg in recent_history.into_iter().rev() {
            let role_label = if msg.role == "user" {
                "用户"
            } else {
                "助手"
            };
            let content = msg.content.trim();
            let short = if content.chars().count() > 200 {
                format!("{}…", content.chars().take(200).collect::<String>())
            } else {
                content.to_string()
            };
            prompt.push_str(&format!("{role_label}：{short}\n"));
        }
    }

    if !references.is_empty() {
        let filtered_references = filter_reference_items(references, 5);
        prompt.push_str("\n【相关记忆】\n");
        if filtered_references.is_empty() {
            prompt.push_str("直接命中的原始记录区分度不高，多数是窗口标题或菜单噪声，请优先参考阶段复盘、意图分布和工作段。\n");
        } else {
            let filtered = filtered_references.into_iter().cloned().collect::<Vec<_>>();
            prompt.push_str(&format_memory_references(&filtered));
        }
        prompt.push('\n');
    }

    if let Some(sessions) = sessions {
        prompt.push_str("\n【工作段】\n");
        prompt.push_str(&summarize_sessions_for_prompt(sessions));
        prompt.push('\n');
    }

    if let Some(intents) = intents {
        prompt.push_str("\n【意图分布】\n");
        prompt.push_str(&summarize_intents_for_prompt(intents));
        prompt.push('\n');
    }

    if let Some(review) = review {
        prompt.push_str("\n【阶段复盘】\n");
        prompt.push_str(&review.markdown);
        prompt.push('\n');
    }

    if let Some(todos) = todos {
        prompt.push_str("\n【待办事项】\n");
        prompt.push_str(&summarize_todos_for_prompt(todos));
        prompt.push('\n');
    }

    prompt.push_str(
        "\n输出要求：\n1. 用中文回答，直接回应用户的问题，不要泛泛而谈。\n2. 使用清晰的 Markdown 排版（标题、列表、加粗等）。\n3. 必须按以下固定结构输出：`## 结论`、`## 结果概览`、`## 过程分析`、`## 依据补充`、`## 复盘总结`。\n4. 整体风格是“先结果，再过程”，每个结论自然带上依据，不要写成审计报告。\n5. 列举时使用无序列表，一行一条。\n6. 不要提及内部分析工具名称。\n7. 不要虚构日期、任务或结果。\n",
    );

    prompt
}

fn build_reference_line(item: &MemorySearchItem) -> String {
    let mut line = format!("- **{}**（{}）", item.title, item.date);
    if let Some(app) = &item.app_name {
        if !app.is_empty() {
            line.push_str(&format!("，{app}"));
        }
    }
    if !item.excerpt.is_empty() {
        let short_excerpt: String = item.excerpt.chars().take(80).collect();
        line.push_str(&format!("：{short_excerpt}"));
        if item.excerpt.chars().count() > 80 {
            line.push('…');
        }
    }
    line
}

fn push_markdown_section(answer: &mut String, title: &str, lines: Vec<String>, empty_text: &str) {
    answer.push_str(title);
    answer.push_str("\n\n");

    if lines.is_empty() {
        answer.push_str(empty_text);
        answer.push_str("\n\n");
        return;
    }

    for line in lines {
        if line.starts_with("- ") || line.starts_with("> ") {
            answer.push_str(&line);
        } else {
            answer.push_str("- ");
            answer.push_str(&line);
        }
        answer.push('\n');
    }
    answer.push('\n');
}

fn collect_reference_lines(references: &[MemorySearchItem], limit: usize) -> Vec<String> {
    filter_reference_items(references, limit)
        .into_iter()
        .map(build_reference_line)
        .collect()
}

fn collect_session_lines(sessions: Option<&[WorkSession]>, limit: usize) -> Vec<String> {
    sessions
        .unwrap_or(&[])
        .iter()
        .take(limit)
        .map(|session| {
            format!(
                "**{}**：{}，主要使用 {}，意图为 {}",
                session.title,
                crate::analysis::format_duration(session.duration),
                session.dominant_app,
                session.intent_label
            )
        })
        .collect()
}

fn collect_intent_lines(intents: Option<&IntentAnalysisResult>, limit: usize) -> Vec<String> {
    intents
        .map(|result| {
            result
                .summary
                .iter()
                .take(limit)
                .map(|item| {
                    format!(
                        "**{}**：{}，{} 段",
                        item.label,
                        crate::analysis::format_duration(item.duration),
                        item.session_count
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_todo_lines(todos: Option<&TodoExtractionResult>, limit: usize) -> Vec<String> {
    todos
        .map(|result| {
            result
                .items
                .iter()
                .take(limit)
                .map(|item| format!("**{}**（{}，{}）", item.title, item.date, item.reason))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_review_lines(review: Option<&WeeklyReviewResult>) -> Vec<String> {
    review
        .map(|result| {
            let mut lines = vec![format!(
                "总投入 {}，活跃 {} 天，累计 {} 个 session，深度工作 {} 段",
                crate::analysis::format_duration(result.total_duration),
                result.active_days,
                result.session_count,
                result.deep_work_sessions
            )];

            lines.extend(
                result
                    .highlights
                    .iter()
                    .take(3)
                    .map(|item| format!("阶段重点：{item}")),
            );

            lines.extend(
                result
                    .risks
                    .iter()
                    .take(2)
                    .map(|item| format!("需要留意：{item}")),
            );

            lines
        })
        .unwrap_or_default()
}

fn build_fallback_assistant_answer(
    question: &str,
    question_kind: AssistantQuestionKind,
    references: &[MemorySearchItem],
    sessions: Option<&[WorkSession]>,
    intents: Option<&IntentAnalysisResult>,
    review: Option<&WeeklyReviewResult>,
    todos: Option<&TodoExtractionResult>,
    _tool_labels: &[String],
) -> String {
    let mut answer = String::new();
    let intent_lines = collect_intent_lines(intents, 3);
    let session_lines = collect_session_lines(sessions, 3);
    let todo_lines = collect_todo_lines(todos, 3);
    let review_lines = collect_review_lines(review);
    let reference_lines = collect_reference_lines(references, 5);

    let conclusion_lines = match question_kind {
        AssistantQuestionKind::StageSummary => {
            if !intent_lines.is_empty() {
                vec![format!(
                    "围绕“{question}”来看，这段时间的主线工作主要集中在 {}。",
                    intent_lines
                        .iter()
                        .take(2)
                        .map(|line| line.replace("- ", ""))
                        .collect::<Vec<_>>()
                        .join("、")
                )]
            } else if !review_lines.is_empty() {
                vec![
                    "当前记录能看到明确的阶段性主线，但细节主要来自复盘摘要而不是细粒度分类。"
                        .to_string(),
                ]
            } else {
                vec!["当前记录不足以完整还原阶段主线，只能给出有限概览。".to_string()]
            }
        }
        AssistantQuestionKind::OutcomeRecap => {
            if !todo_lines.is_empty() {
                vec![
                    "当前更像是“已有阶段结果 + 仍有收口事项”的状态。".to_string(),
                    format!("未完全收口的重点主要有：{}。", todo_lines.join("、")),
                ]
            } else if !review_lines.is_empty() {
                vec!["能确认有阶段性推进结果，但未提取到明确的待收口事项。".to_string()]
            } else {
                vec!["当前记录能看到零散进展，但不足以明确界定阶段结果。".to_string()]
            }
        }
        AssistantQuestionKind::ProcessRecap => {
            if !session_lines.is_empty() {
                vec![
                    "这段时间更像是围绕少数主题持续推进，而不是完全碎片化切换。".to_string(),
                    format!("最典型的推进片段包括：{}。", session_lines.join("、")),
                ]
            } else if !intent_lines.is_empty() {
                vec!["时间投入方向是可见的，但缺少足够的连续工作段来还原完整过程。".to_string()]
            } else {
                vec!["当前记录不足以支撑过程复盘，只能看到零散痕迹。".to_string()]
            }
        }
        AssistantQuestionKind::EvidenceQuery => {
            if !reference_lines.is_empty() {
                vec![
                    "当前结论主要来自直接命中的活动记录，以及能对上时间段的 session / 意图摘要。"
                        .to_string(),
                    "下面我会把可回溯的依据按记录、过程和阶段摘要拆开列出。".to_string(),
                ]
            } else {
                vec!["当前没有足够的直接命中记录，因此依据链条偏弱。".to_string()]
            }
        }
    };
    push_markdown_section(
        &mut answer,
        "## 结论",
        conclusion_lines,
        "暂无足够记录支撑结论。",
    );

    let overview_lines = match question_kind {
        AssistantQuestionKind::StageSummary => {
            let mut lines = review_lines.clone();
            lines.extend(intent_lines.clone());
            lines
        }
        AssistantQuestionKind::OutcomeRecap => {
            let mut lines = review_lines.clone();
            lines.extend(todo_lines.clone());
            lines
        }
        AssistantQuestionKind::ProcessRecap => {
            let mut lines = intent_lines.clone();
            lines.extend(session_lines.clone());
            lines
        }
        AssistantQuestionKind::EvidenceQuery => {
            let mut lines = reference_lines.clone();
            lines.extend(review_lines.iter().take(2).cloned());
            lines
        }
    };
    push_markdown_section(
        &mut answer,
        "## 结果概览",
        overview_lines,
        "暂无足够记录形成结果概览。",
    );

    let process_lines = match question_kind {
        AssistantQuestionKind::StageSummary | AssistantQuestionKind::OutcomeRecap => {
            let mut lines = session_lines.clone();
            lines.extend(intent_lines.clone());
            lines
        }
        AssistantQuestionKind::ProcessRecap => {
            let mut lines = session_lines.clone();
            lines.extend(review_lines.iter().take(2).cloned());
            lines
        }
        AssistantQuestionKind::EvidenceQuery => {
            let mut lines = session_lines.clone();
            lines.extend(intent_lines.clone());
            lines
        }
    };
    push_markdown_section(
        &mut answer,
        "## 过程分析",
        process_lines,
        "暂无足够的过程记录可用于复盘。",
    );

    let evidence_lines = if !reference_lines.is_empty() {
        reference_lines
    } else if references.iter().any(is_low_signal_reference) {
        vec![
            "直接命中的原始记录区分度不高，当前不展开逐条标题，以免窗口壳和菜单词干扰判断。"
                .to_string(),
        ]
    } else if !review_lines.is_empty() {
        review_lines.iter().take(3).cloned().collect()
    } else {
        Vec::new()
    };
    push_markdown_section(
        &mut answer,
        "## 依据补充",
        evidence_lines,
        "当前没有检索到可直接引用的记录依据。",
    );

    let recap_lines = match question_kind {
        AssistantQuestionKind::StageSummary => vec![
            "整体看，这更像是围绕同一条主线持续推进，而不是大范围切题。".to_string(),
            "如果后续继续追问某个模块，我可以再把对应记录单独展开。".to_string(),
        ],
        AssistantQuestionKind::OutcomeRecap => vec![
            "当前更适合把它理解为“阶段推进中”，而不是“已经全部收口”。".to_string(),
            "记录能说明结果轮廓，但具体完成定义仍取决于后续补充追问。".to_string(),
        ],
        AssistantQuestionKind::ProcessRecap => vec![
            "从可见记录看，过程脉络比单点结果更清晰。".to_string(),
            "如果需要，我可以继续把关键时间段按先后顺序串起来。".to_string(),
        ],
        AssistantQuestionKind::EvidenceQuery => vec![
            "当前结论并不是凭空概括，而是来自直接记录命中与阶段摘要的交叉印证。".to_string(),
            "如果你要继续核对，我更适合沿着具体记录或具体时间段往下展开。".to_string(),
        ],
    };
    push_markdown_section(
        &mut answer,
        "## 复盘总结",
        recap_lines,
        "暂无进一步复盘总结。",
    );

    answer
}

fn build_assistant_cards(
    sessions: Option<&[WorkSession]>,
    intents: Option<&IntentAnalysisResult>,
    review: Option<&WeeklyReviewResult>,
    todos: Option<&TodoExtractionResult>,
) -> Vec<AssistantCard> {
    let mut cards = Vec::new();

    if let Some(review) = review {
        cards.push(AssistantCard {
            kind: "review".to_string(),
            title: "阶段复盘".to_string(),
            content: serde_json::json!({
                "totalDuration": review.total_duration,
                "activeDays": review.active_days,
                "sessionCount": review.session_count,
                "deepWorkSessions": review.deep_work_sessions,
                "highlights": review.highlights,
                "risks": review.risks,
            }),
        });
    }

    if let Some(intents) = intents {
        if !intents.summary.is_empty() {
            cards.push(AssistantCard {
                kind: "intents".to_string(),
                title: "意图分布".to_string(),
                content: serde_json::json!({
                    "items": intents.summary.iter().take(6).map(|item| serde_json::json!({
                        "label": item.label,
                        "duration": item.duration,
                        "sessionCount": item.session_count,
                    })).collect::<Vec<_>>(),
                }),
            });
        }
    }

    if let Some(todos) = todos {
        if !todos.items.is_empty() {
            cards.push(AssistantCard {
                kind: "todos".to_string(),
                title: "待办候选".to_string(),
                content: serde_json::json!({
                    "items": todos.items.iter().take(8).map(|item| serde_json::json!({
                        "title": item.title,
                        "date": item.date,
                        "sourceApp": item.source_app,
                        "confidence": item.confidence,
                        "reason": item.reason,
                    })).collect::<Vec<_>>(),
                }),
            });
        }
    }

    if let Some(sessions) = sessions {
        if !sessions.is_empty() {
            cards.push(AssistantCard {
                kind: "sessions".to_string(),
                title: "代表性 Session".to_string(),
                content: serde_json::json!({
                    "items": sessions.iter().take(5).map(|session| serde_json::json!({
                        "title": session.title,
                        "date": session.date,
                        "duration": session.duration,
                        "dominantApp": session.dominant_app,
                        "intentLabel": session.intent_label,
                    })).collect::<Vec<_>>(),
                }),
            });
        }
    }

    cards
}

async fn generate_text_answer_with_model(
    model_config: &ModelConfig,
    system_prompt: &str,
    prompt: &str,
) -> Result<String, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    match model_config.provider {
        AiProvider::Ollama => {
            let response = client
                .post(format!("{}/api/chat", model_config.endpoint))
                .json(&serde_json::json!({
                    "model": model_config.model,
                    "messages": [
                        {
                            "role": "system",
                            "content": system_prompt
                        },
                        {
                            "role": "user",
                            "content": prompt
                        }
                    ],
                    "stream": false
                }))
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(AppError::Analysis(format!(
                    "Ollama 记忆问答失败: {}",
                    response.status()
                )));
            }

            let result: serde_json::Value = response.json().await?;
            let answer = result["message"]["content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            if answer.is_empty() {
                return Err(AppError::Analysis("Ollama 返回空内容".to_string()));
            }
            Ok(answer)
        }
        AiProvider::Claude => {
            let api_key = model_config.api_key.as_deref().unwrap_or("");
            if api_key.is_empty() {
                return Err(AppError::Analysis("Claude API Key 未配置".to_string()));
            }

            let response = client
                .post(format!("{}/messages", model_config.endpoint))
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&serde_json::json!({
                    "model": model_config.model,
                    "max_tokens": 1600,
                    "system": system_prompt,
                    "messages": [
                        {
                            "role": "user",
                            "content": prompt
                        }
                    ]
                }))
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(AppError::Analysis(format!(
                    "Claude 记忆问答失败: {error_text}"
                )));
            }

            let result: serde_json::Value = response.json().await?;
            let answer = result["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            if answer.is_empty() {
                return Err(AppError::Analysis("Claude 返回空内容".to_string()));
            }
            Ok(answer)
        }
        AiProvider::Gemini => {
            let api_key = model_config.api_key.as_deref().unwrap_or("");
            if api_key.is_empty() {
                return Err(AppError::Analysis("Gemini API Key 未配置".to_string()));
            }

            let response = client
                .post(format!(
                    "{}/models/{}:generateContent?key={}",
                    model_config.endpoint, model_config.model, api_key
                ))
                .json(&serde_json::json!({
                    "contents": [{
                        "parts": [{
                            "text": format!("{}\n\n{}", system_prompt, prompt)
                        }]
                    }],
                    "generationConfig": {
                        "temperature": 0.2,
                        "maxOutputTokens": 1600
                    }
                }))
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(AppError::Analysis(format!(
                    "Gemini 记忆问答失败: {error_text}"
                )));
            }

            let result: serde_json::Value = response.json().await?;
            let answer = result["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            if answer.is_empty() {
                return Err(AppError::Analysis("Gemini 返回空内容".to_string()));
            }
            Ok(answer)
        }
        _ => {
            let mut request = client
                .post(format!("{}/chat/completions", model_config.endpoint))
                .json(&serde_json::json!({
                    "model": model_config.model,
                    "messages": [
                        {
                            "role": "system",
                            "content": system_prompt
                        },
                        {
                            "role": "user",
                            "content": prompt
                        }
                    ],
                    "max_tokens": 1600,
                    "temperature": 0.2
                }));

            if let Some(api_key) = &model_config.api_key {
                if !api_key.is_empty() {
                    request = request.header("Authorization", format!("Bearer {api_key}"));
                }
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(AppError::Analysis(format!(
                    "OpenAI 兼容记忆问答失败: {error_text}"
                )));
            }

            let result: serde_json::Value = response.json().await?;
            let answer = result["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .trim()
                .to_string();
            if answer.is_empty() {
                return Err(AppError::Analysis("模型返回空内容".to_string()));
            }
            Ok(answer)
        }
    }
}

async fn generate_memory_answer_with_model(
    model_config: &ModelConfig,
    question: &str,
    references: &[MemorySearchItem],
) -> Result<String, AppError> {
    generate_text_answer_with_model(
        model_config,
        "你是一个严谨的个人工作记忆助手，只能基于提供的记录作答，请用中文回答。",
        &build_memory_answer_prompt(question, references),
    )
    .await
}

fn update_settings_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("update_settings.json")
}

fn load_update_settings_from_dir(data_dir: &Path) -> Result<UpdateSettings, AppError> {
    let settings_path = update_settings_path(data_dir);

    if !settings_path.exists() {
        return Ok(UpdateSettings::default());
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| AppError::Unknown(format!("读取更新设置失败: {e}")))?;

    serde_json::from_str(&content).map_err(|e| AppError::Unknown(format!("解析更新设置失败: {e}")))
}

fn save_update_settings_to_dir(data_dir: &Path, settings: &UpdateSettings) -> Result<(), AppError> {
    let settings_path = update_settings_path(data_dir);
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| AppError::Unknown(format!("序列化更新设置失败: {e}")))?;

    std::fs::write(&settings_path, content)
        .map_err(|e| AppError::Unknown(format!("保存更新设置失败: {e}")))?;

    Ok(())
}

fn should_check_for_updates(settings: &UpdateSettings) -> bool {
    if !settings.auto_check {
        return false;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let interval_hours = if settings.check_interval_hours > 0 {
        settings.check_interval_hours
    } else {
        DEFAULT_UPDATE_CHECK_INTERVAL_HOURS
    };
    let elapsed_hours = now.saturating_sub(settings.last_check_time) / 3600;

    elapsed_hours >= interval_hours
}

async fn check_updater_json(client: &reqwest::Client) -> Result<GithubUpdateInfo, AppError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let mut last_error: Option<String> = None;

    for endpoint in UPDATER_JSON_ENDPOINTS {
        let response = match client.get(*endpoint).send().await {
            Ok(response) => response,
            Err(error) => {
                last_error = Some(format!("{endpoint}: {error}"));
                continue;
            }
        };

        if !response.status().is_success() {
            last_error = Some(format!("{endpoint}: HTTP {}", response.status()));
            continue;
        }

        let updater = match response.json::<UpdaterJsonResponse>().await {
            Ok(updater) => updater,
            Err(error) => {
                last_error = Some(format!("{endpoint}: 解析 updater.json 失败: {error}"));
                continue;
            }
        };

        let latest_version = normalize_version(&updater.version).to_string();
        let has_update = compare_versions(&current_version, &latest_version) == Ordering::Less;

        return Ok(GithubUpdateInfo {
            current_version,
            latest_version,
            available: has_update,
            auto_update_ready: true,
            release_url: GITHUB_LATEST_RELEASE_PAGE.to_string(),
            body: updater.notes,
            source: Some((*endpoint).to_string()),
        });
    }

    Err(AppError::Unknown(last_error.unwrap_or_else(|| {
        "所有 updater.json 更新源都不可用".to_string()
    })))
}

/// 获取今日统计
#[tauri::command]
pub async fn get_today_stats(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DailyStats, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    // 使用配置的工作时间
    let mut stats = state.database.get_daily_stats_with_work_time(
        &today,
        state.config.work_start_hour,
        state.config.work_end_hour,
        state.config.work_start_minute,
        state.config.work_end_minute,
    )?;

    // 过滤掉被隐私规则设置为 Ignored 的应用
    use crate::config::PrivacyLevel;
    let ignored_apps: Vec<_> = state
        .config
        .privacy
        .app_rules
        .iter()
        .filter(|r| r.level == PrivacyLevel::Ignored)
        .map(|r| r.app_name.to_lowercase())
        .collect();

    if !ignored_apps.is_empty() {
        // 过滤 app_usage
        let filtered_app_usage: Vec<_> = stats
            .app_usage
            .into_iter()
            .filter(|app| {
                let app_lower = app.app_name.to_lowercase();
                !ignored_apps
                    .iter()
                    .any(|ignored| app_lower.contains(ignored) || ignored.contains(&app_lower))
            })
            .collect();

        // 重新计算总时长
        let filtered_duration: i64 = filtered_app_usage.iter().map(|a| a.duration).sum();
        stats.total_duration = filtered_duration;
        stats.app_usage = filtered_app_usage;

        // 过滤 browser_usage
        stats.browser_usage.retain(|b| {
            let browser_lower = b.browser_name.to_lowercase();
            !ignored_apps
                .iter()
                .any(|ignored| browser_lower.contains(ignored) || ignored.contains(&browser_lower))
        });

        // 重新计算浏览器时长
        stats.browser_duration = stats.browser_usage.iter().map(|b| b.duration).sum();

        // 办公时长不能超过总时长（隐私过滤后的）
        if stats.work_time_duration > stats.total_duration {
            stats.work_time_duration = stats.total_duration;
        }
    }

    Ok(stats)
}

/// 获取指定日期的统计
#[tauri::command]
pub async fn get_daily_stats(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<DailyStats, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    // 使用配置的工作时间
    state.database.get_daily_stats_with_work_time(
        &date,
        state.config.work_start_hour,
        state.config.work_end_hour,
        state.config.work_start_minute,
        state.config.work_end_minute,
    )
}

/// 获取指定日期的时间线
#[tauri::command]
pub async fn get_timeline(
    date: String,
    limit: Option<u32>,
    offset: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Activity>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = state.database.get_timeline(&date, limit, offset)?;
    let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);
    let filtered = filter_activities_by_privacy(activities, &ignored_apps, &excluded_domains);

    if !ignored_apps.is_empty() || !excluded_domains.is_empty() {
        log::info!(
            "隐私过滤: 需过滤应用 {:?}, 域名 {:?}，结果 {} 条",
            ignored_apps,
            excluded_domains,
            filtered.len()
        );
    }

    Ok(filtered)
}

/// 从 URL 中提取域名
fn extract_domain(url: &str) -> String {
    let without_protocol = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    without_protocol
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn collect_privacy_filters(state: &AppState) -> (Vec<String>, Vec<String>) {
    use crate::config::PrivacyLevel;

    let ignored_apps = state
        .config
        .privacy
        .app_rules
        .iter()
        .filter(|rule| rule.level == PrivacyLevel::Ignored)
        .map(|rule| rule.app_name.to_lowercase())
        .collect::<Vec<_>>();

    let excluded_domains = state
        .config
        .privacy
        .excluded_domains
        .iter()
        .map(|domain| extract_domain(domain))
        .filter(|domain| !domain.is_empty())
        .collect::<Vec<_>>();

    (ignored_apps, excluded_domains)
}

fn filter_activities_by_privacy(
    activities: Vec<Activity>,
    ignored_apps: &[String],
    excluded_domains: &[String],
) -> Vec<Activity> {
    let no_app_filter = ignored_apps.is_empty();
    let no_domain_filter = excluded_domains.is_empty();

    if no_app_filter && no_domain_filter {
        return activities;
    }

    activities
        .into_iter()
        .filter(|activity| {
            let app_lower = activity.app_name.to_lowercase();
            if !no_app_filter
                && ignored_apps
                    .iter()
                    .any(|ignored| app_lower.contains(ignored) || ignored.contains(&app_lower))
            {
                return false;
            }

            if !no_domain_filter {
                if let Some(url) = &activity.browser_url {
                    let domain = extract_domain(url);
                    if excluded_domains
                        .iter()
                        .any(|excluded| domain.contains(excluded) || excluded.contains(&domain))
                    {
                        return false;
                    }
                }
            }

            true
        })
        .collect()
}

fn load_filtered_activities_in_range(
    state: &AppState,
    date_from: Option<&str>,
    date_to: Option<&str>,
    limit: usize,
) -> Result<Vec<Activity>, AppError> {
    let activities = state
        .database
        .get_activities_in_range(date_from, date_to, limit)?;
    let (ignored_apps, excluded_domains) = collect_privacy_filters(state);
    Ok(filter_activities_by_privacy(
        activities,
        &ignored_apps,
        &excluded_domains,
    ))
}

/// 获取单个活动（用于刷新详情页，获取最新 OCR 结果）
#[tauri::command]
pub async fn get_activity(
    id: i64,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<Activity>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.get_activity_by_id(id)
}

/// 搜索工作记忆
#[tauri::command]
pub async fn search_memory(
    query: String,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<MemorySearchItem>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.search_memory(
        &query,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(20) as usize,
    )
}

/// 基于工作记忆回答问题
#[tauri::command]
pub async fn ask_memory(
    question: String,
    date_from: Option<String>,
    date_to: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<MemoryAnswer, AppError> {
    let (model_config, references) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let references =
            state
                .database
                .search_memory(&question, date_from.as_deref(), date_to.as_deref(), 8)?;
        (state.config.text_model.clone(), references)
    };

    if references.is_empty() {
        return Ok(MemoryAnswer {
            answer: build_fallback_memory_answer(&question, &references),
            references,
            used_ai: false,
            model_name: None,
        });
    }

    if is_text_model_available(&model_config) {
        match generate_memory_answer_with_model(&model_config, &question, &references).await {
            Ok(answer) => {
                return Ok(MemoryAnswer {
                    answer,
                    references,
                    used_ai: true,
                    model_name: Some(model_config.model),
                });
            }
            Err(error) => {
                log::warn!("记忆问答 AI 生成失败，回退基础模式: {error}");
            }
        }
    }

    Ok(MemoryAnswer {
        answer: build_fallback_memory_answer(&question, &references),
        references,
        used_ai: false,
        model_name: None,
    })
}

/// 统一工作助手
#[tauri::command]
pub async fn chat_work_assistant(
    question: String,
    history: Option<Vec<AssistantChatMessage>>,
    model_config: Option<ModelConfig>,
    date_from: Option<String>,
    date_to: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<AssistantAnswer, AppError> {
    let trimmed_question = question.trim().to_string();
    let history = history.unwrap_or_default();
    if trimmed_question.is_empty() {
        return Ok(AssistantAnswer {
            answer: "请输入你想问的问题。".to_string(),
            references: Vec::new(),
            used_ai: false,
            model_name: None,
            tool_labels: vec!["记忆检索".to_string()],
            cards: Vec::new(),
        });
    }

    // 时间范围：前端传入优先，否则从问题中自动提取
    let (date_from, date_to) = if date_from.is_some() || date_to.is_some() {
        (date_from, date_to)
    } else {
        let (auto_from, auto_to) = parse_temporal_range(&trimmed_question);
        (auto_from, auto_to)
    };

    let reasoning_mode = assistant_reasoning_mode(model_config.as_ref());
    let question_kind =
        detect_assistant_question_kind_with_mode(&trimmed_question, &history, reasoning_mode);
    let tools = detect_assistant_tools_with_history(&trimmed_question, &history, reasoning_mode);
    let search_query = build_contextual_query(&trimmed_question, &history);
    let (references, sessions, intents, review, todos) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let references = state.database.search_memory(
            &search_query,
            date_from.as_deref(),
            date_to.as_deref(),
            8,
        )?;

        let needs_activity_data = tools.iter().any(|tool| {
            matches!(
                tool,
                AssistantTool::Sessions
                    | AssistantTool::Intents
                    | AssistantTool::Review
                    | AssistantTool::Todos
            )
        });

        let activities = if needs_activity_data {
            Some(load_filtered_activities_in_range(
                &state,
                date_from.as_deref(),
                date_to.as_deref(),
                5000,
            )?)
        } else {
            None
        };

        let sessions = if tools.contains(&AssistantTool::Sessions) {
            activities.as_ref().map(|items| build_work_sessions(items))
        } else {
            None
        };

        let intents = if tools.contains(&AssistantTool::Intents) {
            activities.as_ref().map(|items| analyze_intents(items))
        } else {
            None
        };

        let review = if tools.contains(&AssistantTool::Review) {
            activities
                .as_ref()
                .map(|items| build_weekly_review(items, date_from.as_deref(), date_to.as_deref()))
        } else {
            None
        };

        let todos = if tools.contains(&AssistantTool::Todos) {
            activities.as_ref().map(|items| extract_todos(items))
        } else {
            None
        };

        (references, sessions, intents, review, todos)
    };

    let tool_labels = tools
        .iter()
        .map(|tool| tool.label().to_string())
        .collect::<Vec<_>>();
    let cards = build_assistant_cards(
        sessions.as_deref(),
        intents.as_ref(),
        review.as_ref(),
        todos.as_ref(),
    );

    // model_config: None = basic template (no AI), Some = AI enhanced
    if let Some(ref ai_model) = model_config {
        if is_text_model_available(ai_model) {
            let prompt = build_assistant_prompt(
                &trimmed_question,
                question_kind,
                &history,
                date_from.as_deref(),
                date_to.as_deref(),
                &references,
                sessions.as_deref(),
                intents.as_ref(),
                review.as_ref(),
                todos.as_ref(),
            );

            let sys = "你是 Work Review 的工作助手。你只能基于给定记录回答。请用中文回答，直接回应用户问题，先给结论再给依据。不要提及内部分析步骤，不要编造不存在的事实。";

            match generate_text_answer_with_model(ai_model, sys, &prompt).await {
                Ok(answer) => {
                    return Ok(AssistantAnswer {
                        answer,
                        references,
                        used_ai: true,
                        model_name: Some(ai_model.model.clone()),
                        tool_labels,
                        cards,
                    });
                }
                Err(error) => {
                    log::warn!("AI generation failed, falling back: {error}");
                }
            }
        }
    }

    Ok(AssistantAnswer {
        answer: build_fallback_assistant_answer(
            &trimmed_question,
            question_kind,
            &references,
            sessions.as_deref(),
            intents.as_ref(),
            review.as_ref(),
            todos.as_ref(),
            &tool_labels,
        ),
        references,
        used_ai: false,
        model_name: None,
        tool_labels,
        cards,
    })
}

/// 获取连续工作 session 聚合结果
#[tauri::command]
pub async fn get_work_sessions(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<WorkSession>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(build_work_sessions(&activities))
}

/// 基于 session 识别主要工作意图
#[tauri::command]
pub async fn recognize_work_intents(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<IntentAnalysisResult, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(analyze_intents(&activities))
}

/// 生成周报 / 阶段复盘
#[tauri::command]
pub async fn generate_weekly_review(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<WeeklyReviewResult, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(build_weekly_review(
        &activities,
        date_from.as_deref(),
        date_to.as_deref(),
    ))
}

/// 提取待跟进事项
#[tauri::command]
pub async fn extract_todo_items(
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<u32>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<TodoExtractionResult, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let activities = load_filtered_activities_in_range(
        &state,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(5000) as usize,
    )?;

    Ok(extract_todos(&activities))
}

/// 生成日报
#[tauri::command]
pub async fn generate_report(
    date: String,
    force: Option<bool>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    // 如果不是强制重新生成，先检查缓存
    if !force.unwrap_or(false) {
        let state_guard = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if let Ok(Some(cached)) = state_guard.database.get_report(&date) {
            log::info!("使用缓存日报: {date}");
            return Ok(cached.content);
        }
    }

    let (config, stats, activities, data_dir) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let stats = state.database.get_daily_stats(&date)?;
        // 生成日报时获取最多 2000 条记录
        let activities = state.database.get_timeline(&date, Some(2000), None)?;
        (
            state.config.clone(),
            stats,
            activities,
            state.data_dir.clone(),
        )
    };

    let avatar_start_state = {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.avatar_generating_report = true;
        let avatar_state = crate::avatar_engine::apply_avatar_opacity(
            crate::avatar_engine::derive_avatar_state(
                &state.avatar_state.app_name,
                "",
                None,
                state.avatar_state.is_idle,
                true,
            ),
            state.config.avatar_opacity,
        );
        state.avatar_state = avatar_state.clone();
        if state.config.avatar_enabled {
            Some(avatar_state)
        } else {
            None
        }
    };

    if let Some(avatar_state) = avatar_start_state.as_ref() {
        crate::avatar_engine::emit_avatar_state(&app, avatar_state);
        crate::avatar_engine::emit_avatar_bubble(
            &app,
            &crate::avatar_engine::AvatarBubblePayload::info("开始整理日报，稍等我一下。"),
        );
    }

    // 创建分析器（使用 text_model 配置）
    let analyzer = crate::analysis::create_analyzer(
        config.ai_mode,
        config.text_model.provider,
        &config.text_model.endpoint,
        &config.text_model.model,
        config.text_model.api_key.as_deref(),
    );

    // 生成报告
    let screenshots_dir = data_dir.join("screenshots");
    let report_result = analyzer
        .generate_report(&date, &stats, &activities, &screenshots_dir)
        .await;

    let avatar_finish_state = {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.avatar_generating_report = false;
        let avatar_state = crate::avatar_engine::apply_avatar_opacity(
            crate::avatar_engine::derive_avatar_state(
                &state.avatar_state.app_name,
                "",
                None,
                state.avatar_state.is_idle,
                false,
            ),
            state.config.avatar_opacity,
        );
        state.avatar_state = avatar_state.clone();
        if state.config.avatar_enabled {
            Some(avatar_state)
        } else {
            None
        }
    };

    if let Some(avatar_state) = avatar_finish_state.as_ref() {
        crate::avatar_engine::emit_avatar_state(&app, avatar_state);
        let bubble = if report_result.is_ok() {
            crate::avatar_engine::AvatarBubblePayload::success("日报整理好了，可以回来看看。")
        } else {
            crate::avatar_engine::AvatarBubblePayload::info("这次日报整理失败了，稍后可以再试。")
        };
        crate::avatar_engine::emit_avatar_bubble(&app, &bubble);
    }

    let report = report_result?;

    // 保存报告
    {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let daily_report = DailyReport {
            date: date.clone(),
            content: report.clone(),
            ai_mode: format!("{:?}", config.ai_mode),
            model_name: Some(config.text_model.model.clone()),
            created_at: chrono::Utc::now().timestamp(),
        };
        state.database.save_report(&daily_report)?;
    }

    Ok(report)
}

/// 获取已保存的日报
#[tauri::command]
pub async fn get_saved_report(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<DailyReport>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.get_report(&date)
}

/// 获取配置
#[tauri::command]
pub async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> Result<AppConfig, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(state.config.clone())
}

pub(crate) fn persist_app_config(
    mut config: AppConfig,
    app: AppHandle,
    state: &Arc<Mutex<AppState>>,
) -> Result<(), AppError> {
    config.normalize();
    let avatar_state = {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;

        // 更新配置
        state.config = config.clone();
        state.storage_manager.update_config(config.storage.clone());

        // 保存到文件
        let config_path = state.config_path.clone();
        config.save(&config_path)?;

        // 更新隐私过滤器
        state.privacy_filter.update_config(&config.privacy);
        state.avatar_state = crate::avatar_engine::apply_avatar_opacity(
            state.avatar_state.clone(),
            config.avatar_opacity,
        );
        state.avatar_state.clone()
    };

    crate::avatar_engine::sync_avatar_window(
        &app,
        config.avatar_enabled,
        config.avatar_scale,
        config.avatar_x.zip(config.avatar_y),
    )
    .map_err(|e| AppError::Unknown(format!("同步桌宠窗口失败: {e}")))?;
    if config.avatar_enabled && !refresh_avatar_state_for_current_window(&app, state) {
        crate::avatar_engine::emit_avatar_state(&app, &avatar_state);
    }
    crate::sync_effective_dock_visibility(&app);
    crate::emit_config_changed(&app, &config);

    log::info!("配置已保存");
    Ok(())
}

/// 保存配置
#[tauri::command]
pub async fn save_config(
    config: AppConfig,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    persist_app_config(config, app, state.inner())
}

fn refresh_avatar_state_for_current_window(app: &AppHandle, state: &Arc<Mutex<AppState>>) -> bool {
    let active_window = match crate::monitor::get_active_window_fast() {
        Ok(window) => window,
        Err(_) => return false,
    };

    let next_avatar_state = {
        let mut state_guard = match state.lock() {
            Ok(guard) => guard,
            Err(e) => {
                log::warn!("刷新桌宠状态时获取状态锁失败: {e}");
                return false;
            }
        };

        if !state_guard.config.avatar_enabled {
            return false;
        }

        let next_state = crate::avatar_engine::apply_avatar_opacity(
            crate::avatar_engine::derive_avatar_state_with_rules(
                &state_guard.config.app_category_rules,
                &active_window.app_name,
                &active_window.window_title,
                active_window.browser_url.as_deref(),
                state_guard.avatar_state.is_idle,
                state_guard.avatar_generating_report,
            ),
            state_guard.config.avatar_opacity,
        );
        state_guard.avatar_state = next_state.clone();
        next_state
    };

    crate::avatar_engine::emit_avatar_state(app, &next_avatar_state);
    true
}

/// 获取更新检查设置
#[tauri::command]
pub async fn get_update_settings(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<UpdateSettings, AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };

    load_update_settings_from_dir(&data_dir)
}

/// 保存更新检查设置
#[tauri::command]
pub async fn save_update_settings(
    settings: UpdateSettings,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };

    save_update_settings_to_dir(&data_dir, &settings)
}

/// 判断当前是否应自动检查更新
#[tauri::command]
pub async fn should_check_updates(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };
    let settings = load_update_settings_from_dir(&data_dir)?;

    Ok(should_check_for_updates(&settings))
}

/// 更新时间检查时间戳
#[tauri::command]
pub async fn update_last_check_time(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };
    let mut settings = load_update_settings_from_dir(&data_dir)?;
    settings.last_check_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    save_update_settings_to_dir(&data_dir, &settings)
}

/// 测试 AI 模型连接
#[tauri::command]
pub async fn test_ai_model(provider_config: AiProviderConfig) -> Result<ModelTestResult, AppError> {
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    let result = match provider_config.provider {
        AiProvider::Ollama => test_ollama(&client, &provider_config).await,
        AiProvider::Gemini => test_gemini(&client, &provider_config).await,
        AiProvider::Claude => test_claude(&client, &provider_config).await,
        // OpenAI 及兼容格式的供应商
        _ => test_openai(&client, &provider_config).await,
    };

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(ModelTestResult {
            success: true,
            message: "连接成功！模型可用。".to_string(),
            response_time_ms: elapsed,
            model_info: Some(info),
        }),
        Err(e) => Ok(ModelTestResult {
            success: false,
            message: format!("连接失败: {e}"),
            response_time_ms: elapsed,
            model_info: None,
        }),
    }
}

/// 测试模型连接（新版，使用 ModelConfig）
#[tauri::command]
pub async fn test_model(model_config: ModelConfig) -> Result<ModelTestResult, AppError> {
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    // 将 ModelConfig 转换为 AiProviderConfig 以复用现有测试逻辑
    let provider_config = AiProviderConfig {
        provider: model_config.provider,
        endpoint: model_config.endpoint,
        api_key: model_config.api_key,
        model: model_config.model,
        vision_model: None,
    };

    let result = match provider_config.provider {
        AiProvider::Ollama => test_ollama(&client, &provider_config).await,
        AiProvider::Gemini => test_gemini(&client, &provider_config).await,
        AiProvider::Claude => test_claude(&client, &provider_config).await,
        // OpenAI 及兼容格式的供应商（硅基流动、DeepSeek、通义千问、智谱、月之暗面、豆包）
        _ if provider_config.provider.is_openai_compatible() => {
            test_openai(&client, &provider_config).await
        }
        // 兜底：默认使用 OpenAI 格式
        _ => test_openai(&client, &provider_config).await,
    };

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(info) => Ok(ModelTestResult {
            success: true,
            message: "连接成功！模型可用。".to_string(),
            response_time_ms: elapsed,
            model_info: Some(info),
        }),
        Err(e) => Ok(ModelTestResult {
            success: false,
            message: format!("连接失败: {e}"),
            response_time_ms: elapsed,
            model_info: None,
        }),
    }
}

/// 测试 Ollama 连接
async fn test_ollama(
    client: &reqwest::Client,
    config: &AiProviderConfig,
) -> Result<String, String> {
    // 1. 先测试服务是否可用
    let tags_url = format!("{}/api/tags", config.endpoint);
    let response = client
        .get(&tags_url)
        .send()
        .await
        .map_err(|e| format!("无法连接到 Ollama 服务: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Ollama 服务返回错误: {}", response.status()));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;

    // 2. 检查模型是否存在于列表中
    let models = data["models"].as_array().ok_or("无法获取模型列表")?;

    let model_exists = models.iter().any(|m| {
        m["name"]
            .as_str()
            .map(|n| n.starts_with(&config.model) || n.contains(&config.model))
            .unwrap_or(false)
    });

    if !model_exists {
        let available: Vec<String> = models
            .iter()
            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
            .take(5)
            .collect();
        return Err(format!(
            "模型 {} 未安装。可用模型: {}",
            config.model,
            available.join(", ")
        ));
    }

    // 3. 实际调用模型生成测试（关键验证步骤）
    let generate_url = format!("{}/api/generate", config.endpoint);
    let test_response = client
        .post(&generate_url)
        .json(&serde_json::json!({
            "model": config.model,
            "prompt": "Hi",
            "stream": false,
            "options": {
                "num_predict": 5  // 只生成5个token，快速测试
            }
        }))
        .send()
        .await
        .map_err(|e| format!("调用模型失败: {e}"))?;

    if !test_response.status().is_success() {
        let error_text = test_response.text().await.unwrap_or_default();
        return Err(format!("模型响应失败: {error_text}"));
    }

    let result: serde_json::Value = test_response
        .json()
        .await
        .map_err(|e| format!("解析模型响应失败: {e}"))?;

    // 检查是否有实际响应
    if result["response"].as_str().is_some() {
        Ok(format!("模型 {} 测试通过，响应正常", config.model))
    } else {
        Err("模型返回空响应".to_string())
    }
}

/// 测试 OpenAI 连接
async fn test_openai(
    client: &reqwest::Client,
    config: &AiProviderConfig,
) -> Result<String, String> {
    let api_key = config.api_key.as_ref().ok_or("未配置 API Key")?;

    let response = client
        .post(format!("{}/chat/completions", config.endpoint))
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "model": config.model,
            "messages": [{"role": "user", "content": "Hello"}],
            "max_tokens": 5,
        }))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    if response.status().is_success() {
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {e}"))?;
        let model_used = data["model"].as_str().unwrap_or(&config.model);
        Ok(format!("模型 {model_used} 响应正常"))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("API 错误: {error_text}"))
    }
}

/// 测试 Google Gemini 连接
async fn test_gemini(
    client: &reqwest::Client,
    config: &AiProviderConfig,
) -> Result<String, String> {
    let api_key = config.api_key.as_ref().ok_or("未配置 API Key")?;

    let url = format!(
        "{}/models/{}:generateContent?key={}",
        config.endpoint, config.model, api_key
    );

    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": "Hello"}]}],
            "generationConfig": {"maxOutputTokens": 10}
        }))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    if response.status().is_success() {
        Ok(format!("Gemini 模型 {} 响应正常", config.model))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("API 错误: {error_text}"))
    }
}

/// 测试 Anthropic Claude 连接
async fn test_claude(
    client: &reqwest::Client,
    config: &AiProviderConfig,
) -> Result<String, String> {
    let api_key = config.api_key.as_ref().ok_or("未配置 API Key")?;

    let response = client
        .post(format!("{}/messages", config.endpoint))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": config.model,
            "max_tokens": 10,
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    if response.status().is_success() {
        Ok(format!("Claude 模型 {} 响应正常", config.model))
    } else {
        let error_text = response.text().await.unwrap_or_default();
        Err(format!("API 错误: {error_text}"))
    }
}

/// 获取支持的 AI 提供商列表
#[tauri::command]
pub async fn get_ai_providers() -> Result<Vec<serde_json::Value>, AppError> {
    Ok(vec![
        serde_json::json!({
            "id": "ollama",
            "name": "Ollama (本地)",
            "description": "在本机运行的开源大模型，数据不出本机",
            "default_endpoint": "http://localhost:11434",
            "default_model": "qwen2.5",
            "requires_api_key": false,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "openai",
            "name": "OpenAI / 兼容API",
            "description": "支持 OpenAI 官方及兼容 API（Azure、Cloudflare 等）",
            "default_endpoint": "https://api.openai.com/v1",
            "default_model": "gpt-4o-mini",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "siliconflow",
            "name": "硅基流动 SiliconFlow",
            "description": "国内高性价比 API，兼容 OpenAI 格式",
            "default_endpoint": "https://api.siliconflow.cn/v1",
            "default_model": "Qwen/Qwen2.5-7B-Instruct",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "deepseek",
            "name": "DeepSeek",
            "description": "国产开源模型，性能强劲，兼容 OpenAI 格式",
            "default_endpoint": "https://api.deepseek.com",
            "default_model": "deepseek-chat",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "qwen",
            "name": "通义千问 Qwen",
            "description": "阿里云通义大模型，兼容 OpenAI 格式",
            "default_endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "default_model": "qwen-turbo",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "zhipu",
            "name": "智谱 ChatGLM",
            "description": "智谱 AI 大模型",
            "default_endpoint": "https://open.bigmodel.cn/api/paas/v4",
            "default_model": "glm-4-flash",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "moonshot",
            "name": "月之暗面 Kimi",
            "description": "Moonshot AI，擅长长文本",
            "default_endpoint": "https://api.moonshot.cn/v1",
            "default_model": "moonshot-v1-8k",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "doubao",
            "name": "火山引擎 豆包",
            "description": "字节跳动大模型",
            "default_endpoint": "https://ark.cn-beijing.volces.com/api/v3",
            "default_model": "doubao-lite-4k",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "gemini",
            "name": "Google Gemini",
            "description": "Google 的 Gemini 系列模型",
            "default_endpoint": "https://generativelanguage.googleapis.com/v1",
            "default_model": "gemini-1.5-flash",
            "requires_api_key": true,
            "supports_vision": false,
        }),
        serde_json::json!({
            "id": "claude",
            "name": "Anthropic Claude",
            "description": "Anthropic 的 Claude 系列模型",
            "default_endpoint": "https://api.anthropic.com/v1",
            "default_model": "claude-3-haiku-20240307",
            "requires_api_key": true,
            "supports_vision": false,
        }),
    ])
}

/// 开始录制
#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_recording = true;
    state.is_paused = false;
    log::info!("开始录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 停止录制
#[tauri::command]
pub async fn stop_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_recording = false;
    state.is_paused = false;
    log::info!("停止录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 暂停录制
#[tauri::command]
pub async fn pause_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_paused = true;
    log::info!("暂停录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 恢复录制
#[tauri::command]
pub async fn resume_recording(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.is_recording = true;
    state.is_paused = false;
    log::info!("恢复录制");
    drop(state);
    crate::emit_recording_state_changed(&app);
    Ok(())
}

/// 获取录制状态
#[tauri::command]
pub async fn get_recording_state(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(bool, bool), AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok((state.is_recording, state.is_paused))
}

/// 获取当前桌宠状态
#[tauri::command]
pub async fn get_avatar_state(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<crate::avatar_engine::AvatarStatePayload, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(state.avatar_state.clone())
}

/// 保存桌宠窗口位置
#[tauri::command]
pub async fn save_avatar_position(
    x: i32,
    y: i32,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let config_path = state.config_path.clone();

    state.config.avatar_x = Some(x);
    state.config.avatar_y = Some(y);
    state.config.save(&config_path)?;

    Ok(())
}

/// 显示主窗口
#[tauri::command]
pub async fn show_main_window(
    app: AppHandle,
    source_window_label: Option<String>,
) -> Result<(), AppError> {
    crate::reveal_main_window(&app, source_window_label.as_deref())
}

/// 获取数据目录
#[tauri::command]
pub async fn get_data_dir(state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(path_for_display(&state.data_dir))
}

/// 获取默认数据目录
#[tauri::command]
pub async fn get_default_data_dir() -> Result<String, AppError> {
    Ok(path_for_display(&crate::default_data_dir()))
}

#[tauri::command]
pub async fn get_runtime_platform() -> Result<String, AppError> {
    Ok(std::env::consts::OS.to_string())
}

#[tauri::command]
pub async fn quit_app_for_update(app: AppHandle) -> Result<(), AppError> {
    app.exit(0);
    Ok(())
}

fn path_for_display(path: &Path) -> String {
    let raw = path.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        raw.strip_prefix(r"\\?\")
            .or_else(|| raw.strip_prefix(r"\??\"))
            .unwrap_or(&raw)
            .to_string()
    }

    #[cfg(not(target_os = "windows"))]
    {
        raw
    }
}

fn is_ignorable_dir_entry(name: &str) -> bool {
    name.starts_with('.') || name == "Thumbs.db"
}

fn is_managed_dir_entry(name: &str) -> bool {
    MANAGED_DATA_ENTRIES.contains(&name)
}

fn to_absolute_path(path: &Path) -> Result<PathBuf, AppError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn ensure_target_dir_ready(target_dir: &Path) -> Result<bool, AppError> {
    std::fs::create_dir_all(target_dir)?;

    let mut has_existing_app_data = false;

    for entry in std::fs::read_dir(target_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if is_ignorable_dir_entry(&name) {
            continue;
        }

        if !is_managed_dir_entry(&name) {
            return Err(AppError::Config(format!(
                "目标目录包含非 Work Review 数据（{}），为避免误覆盖，请选择空目录或旧的数据目录",
                name
            )));
        }

        has_existing_app_data = true;
    }

    if !has_existing_app_data {
        return Ok(false);
    }

    // 目标目录若已存在旧版应用数据，先清空受管条目，再完整覆盖为当前数据。
    for entry_name in MANAGED_DATA_ENTRIES {
        let path = target_dir.join(entry_name);
        if !path.exists() {
            continue;
        }

        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }

    Ok(true)
}

/// 切换数据目录，并迁移当前数据
#[tauri::command]
pub async fn change_data_dir(
    target_dir: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let requested_dir = target_dir.trim();
    if requested_dir.is_empty() {
        return Err(AppError::Config("目标目录不能为空".to_string()));
    }

    let requested_path = to_absolute_path(Path::new(requested_dir))?;
    let current_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state
            .data_dir
            .canonicalize()
            .unwrap_or_else(|_| state.data_dir.clone())
    };

    if requested_path == current_dir {
        return Ok(serde_json::json!({
            "dataDir": current_dir.to_string_lossy().to_string(),
            "copiedFiles": 0,
            "message": "数据目录未变化",
        }));
    }

    if requested_path.starts_with(&current_dir) || current_dir.starts_with(&requested_path) {
        return Err(AppError::Config(
            "新旧数据目录不能互为父子目录，请选择独立目录".to_string(),
        ));
    }

    let target_dir = {
        std::fs::create_dir_all(&requested_path)?;
        requested_path
            .canonicalize()
            .unwrap_or_else(|_| requested_path.clone())
    };

    let replaced_existing_data = ensure_target_dir_ready(&target_dir)?;
    let copied_files = crate::copy_dir_contents(&current_dir, &target_dir, true)?;

    let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let config = state.config.clone();
    let config_path = target_dir.join("config.json");
    config.save(&config_path)?;

    let database = Database::new(&target_dir.join("workreview.db"))?;
    let privacy_filter = PrivacyFilter::from_config(&config.privacy);
    let screenshot_service = ScreenshotService::new(&target_dir);
    let storage_manager = StorageManager::new(&target_dir, config.storage.clone());

    crate::save_data_dir_preference(&target_dir)?;

    state.database = database;
    state.privacy_filter = privacy_filter;
    state.screenshot_service = screenshot_service;
    state.storage_manager = storage_manager;
    state.data_dir = target_dir.clone();
    state.config_path = config_path;

    log::info!("数据目录已切换到: {:?}", target_dir);

    Ok(serde_json::json!({
        "dataDir": target_dir.to_string_lossy().to_string(),
        "copiedFiles": copied_files,
        "replacedExistingData": replaced_existing_data,
        "message": format!(
            "数据目录已更新，已迁移 {} 个文件{}",
            copied_files,
            if replaced_existing_data { "，并覆盖旧目录中的 Work Review 数据" } else { "" }
        ),
    }))
}

/// 基于 updater.json 优先检查更新；若自动更新元数据暂未就绪，则回退到 GitHub Release API。
#[tauri::command]
pub async fn check_github_update() -> Result<GithubUpdateInfo, AppError> {
    let client = reqwest::Client::builder()
        .user_agent("WorkReview-Updater")
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Unknown(format!("创建更新检查客户端失败: {e}")))?;

    if let Ok(update_info) = check_updater_json(&client).await {
        return Ok(update_info);
    }

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let release = client
        .get(GITHUB_LATEST_RELEASE_API)
        .header(reqwest::header::USER_AGENT, "WorkReview-Updater")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json::<GithubReleaseResponse>()
        .await?;

    let latest_version = normalize_version(&release.tag_name).to_string();
    let has_update = compare_versions(&current_version, &latest_version) == Ordering::Less;

    if !has_update {
        return Ok(GithubUpdateInfo {
            current_version,
            latest_version,
            available: false,
            auto_update_ready: false,
            release_url: release.html_url,
            body: release.body,
            source: Some("github-release-api".to_string()),
        });
    }

    Ok(GithubUpdateInfo {
        current_version,
        latest_version,
        available: true,
        auto_update_ready: false,
        release_url: release.html_url,
        body: release.body,
        source: Some("github-release-api".to_string()),
    })
}

/// 逐个尝试更新源进行在线更新，避免某个代理只返回 updater.json 但下载失败时直接中断。
#[tauri::command]
pub async fn download_and_install_github_update(
    app: AppHandle,
    expected_version: Option<String>,
) -> Result<GithubUpdateInstallResult, AppError> {
    let mut attempted_sources = Vec::new();
    let mut failures = Vec::new();

    for endpoint in UPDATER_JSON_ENDPOINTS {
        let source_label = update_source_label(endpoint);
        attempted_sources.push(source_label.clone());

        emit_update_status(
            &app,
            "checking",
            format!("正在检查更新源 {source_label}..."),
            Some(source_label.clone()),
            expected_version.clone(),
            None,
            None,
            None,
        );

        let manifest_candidates =
            build_updater_manifest_candidates(endpoint, expected_version.as_deref());

        let mut update = None;
        let mut last_check_error = None;

        for manifest_endpoint in manifest_candidates {
            let endpoint_url = Url::parse(&manifest_endpoint).map_err(|e| {
                AppError::Unknown(format!("解析更新源失败 ({manifest_endpoint}): {e}"))
            })?;

            let updater = match app
                .updater_builder()
                .endpoints(vec![endpoint_url])
                .map_err(|e| AppError::Unknown(format!("配置更新源失败 ({source_label}): {e}")))?
                .timeout(Duration::from_secs(20))
                .configure_client(|client| {
                    client
                        .connect_timeout(Duration::from_secs(10))
                        .user_agent("WorkReview-Updater")
                })
                .build()
            {
                Ok(updater) => updater,
                Err(error) => {
                    last_check_error = Some(format!("{source_label}: 构建更新器失败: {error}"));
                    continue;
                }
            };

            match updater.check().await {
                Ok(Some(found_update)) => {
                    update = Some(found_update);
                    last_check_error = None;
                    break;
                }
                Ok(None) => {
                    last_check_error = Some(format!("{source_label}: 未返回可安装的更新包"));
                }
                Err(error) => {
                    last_check_error = Some(format!("{source_label}: 检查更新失败: {error}"));
                }
            }
        }

        let Some(update) = update else {
            if let Some(error) = last_check_error {
                failures.push(error);
            } else {
                failures.push(format!("{source_label}: 未返回可安装的更新包"));
            }
            continue;
        };

        if let Some(expected) = expected_version.as_deref() {
            if compare_versions(&update.version, expected) == Ordering::Less {
                failures.push(format!(
                    "{source_label}: 返回版本 {}，低于目标版本 {}",
                    update.version, expected
                ));
                continue;
            }
        }

        emit_update_status(
            &app,
            "found",
            format!(
                "发现新版本 {}，准备从 {source_label} 下载...",
                update.version
            ),
            Some(source_label.clone()),
            Some(update.version.clone()),
            None,
            None,
            None,
        );

        let progress_app = app.clone();
        let progress_source = source_label.clone();
        let progress_version = update.version.clone();
        let mut downloaded_bytes = 0_u64;

        let finish_app = app.clone();
        let finish_source = source_label.clone();
        let finish_version = update.version.clone();

        match update
            .download_and_install(
                move |chunk_length, total_bytes| {
                    downloaded_bytes += chunk_length as u64;
                    let percent = total_bytes.and_then(|total| {
                        if total == 0 {
                            None
                        } else {
                            Some(((downloaded_bytes * 100) / total).min(100))
                        }
                    });

                    let message = if let Some(percent) = percent {
                        format!("正在下载更新 {percent}%（{progress_source}）")
                    } else {
                        let mb = ((downloaded_bytes as f64) / 1024.0 / 1024.0).max(0.1);
                        format!("正在下载更新 {:.1} MB（{}）", mb, progress_source)
                    };

                    emit_update_status(
                        &progress_app,
                        "downloading",
                        message,
                        Some(progress_source.clone()),
                        Some(progress_version.clone()),
                        Some(downloaded_bytes),
                        total_bytes,
                        percent,
                    );
                },
                move || {
                    emit_update_status(
                        &finish_app,
                        "installing",
                        format!("下载完成，正在安装（{}）...", finish_source),
                        Some(finish_source.clone()),
                        Some(finish_version.clone()),
                        None,
                        None,
                        Some(100),
                    );
                },
            )
            .await
        {
            Ok(()) => {
                emit_update_status(
                    &app,
                    "completed",
                    format!("更新安装完成，来源 {source_label}"),
                    Some(source_label.clone()),
                    Some(update.version.clone()),
                    None,
                    None,
                    Some(100),
                );

                return Ok(GithubUpdateInstallResult {
                    updated: true,
                    available: true,
                    version: Some(update.version),
                    source: Some(source_label),
                    message: "在线更新已完成".to_string(),
                    attempted_sources,
                });
            }
            Err(error) => {
                failures.push(format!("{source_label}: 下载或安装失败: {error}"));
                emit_update_status(
                    &app,
                    "retrying",
                    format!("源 {source_label} 更新失败，准备尝试下一个源..."),
                    Some(source_label),
                    Some(update.version.clone()),
                    None,
                    None,
                    None,
                );
            }
        }
    }

    let message = if failures.is_empty() {
        if let Some(expected) = expected_version.as_deref() {
            format!("未找到可用于版本 {expected} 的在线更新源")
        } else {
            "当前未发现可安装的在线更新".to_string()
        }
    } else {
        format!("在线更新失败，已尝试全部更新源：{}", failures.join("；"))
    };

    emit_update_status(
        &app,
        "failed",
        message.clone(),
        None,
        expected_version.clone(),
        None,
        None,
        None,
    );

    Err(AppError::Unknown(message))
}

/// 在系统文件管理器中打开数据目录
/// plugin-shell 的 open 对本地路径在部分平台不可靠，改用系统命令直接打开
#[tauri::command]
pub async fn open_data_dir(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };

    // 目录不存在时先创建，避免打开失败
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| AppError::Unknown(format!("创建数据目录失败: {e}")))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开数据目录失败: {e}")))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开数据目录失败: {e}")))?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("打开数据目录失败: {e}")))?;
    }

    Ok(())
}

/// 获取截图缩略图
#[tauri::command]
pub async fn get_screenshot_thumbnail(
    path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let full_path = state.data_dir.join(&path);
    state
        .screenshot_service
        .generate_thumbnail_base64(&full_path, 400)
}

/// 获取高分辨率截图（用于详情弹窗，1200px）
#[tauri::command]
pub async fn get_screenshot_full(
    path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let full_path = state.data_dir.join(&path);
    state
        .screenshot_service
        .generate_thumbnail_base64(&full_path, 1200)
}

/// 手动执行一次截屏
#[tauri::command]
pub async fn take_screenshot(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Activity, AppError> {
    let (
        screenshot_result,
        app_name,
        window_title,
        browser_url,
        category,
        semantic_category,
        semantic_confidence,
        relative_path,
        executable_path,
    ) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;

        // 获取当前活动窗口
        let active_window = crate::monitor::get_active_window()?;

        // 检查隐私过滤
        if state.privacy_filter.check_privacy_full(
            &active_window.app_name,
            &active_window.window_title,
            active_window.browser_url.as_deref(),
        ) == crate::privacy::PrivacyAction::Skip
        {
            return Err(AppError::Privacy("当前窗口被隐私规则过滤".to_string()));
        }

        // 执行截屏
        let result = state
            .screenshot_service
            .capture_for_window(Some(&active_window))?;
        let relative_path = state.screenshot_service.get_relative_path(&result.path);
        let classification = crate::resolve_activity_classification(
            &state.config,
            &active_window.app_name,
            &active_window.window_title,
            active_window.browser_url.as_deref(),
        );

        (
            result,
            active_window.app_name,
            active_window.window_title,
            active_window.browser_url,
            classification.base_category,
            classification.semantic_category,
            classification.confidence,
            relative_path,
            active_window.executable_path,
        )
    };

    // 创建活动记录
    let activity = Activity {
        id: None,
        timestamp: screenshot_result.timestamp,
        app_name,
        window_title,
        screenshot_path: relative_path,
        ocr_text: None,
        category,
        duration: 30,
        browser_url,
        executable_path,
        semantic_category: Some(semantic_category),
        semantic_confidence: Some(i32::from(semantic_confidence)),
    };

    // 保存到数据库
    {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.database.insert_activity(&activity)?;
    }

    Ok(activity)
}

/// 获取历史应用列表（从数据库）
#[tauri::command]
pub async fn get_recent_apps(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, AppError> {
    // 获取最多 50 个历史应用
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    state.database.get_recent_apps(50)
}

#[tauri::command]
pub async fn get_app_category_overview(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<AppCategoryOverviewItem>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let overview = state.database.get_app_category_overview()?;

    Ok(overview
        .into_iter()
        .map(|item| {
            let app_name = item.app_name;
            let override_category =
                crate::monitor::find_category_override(&state.config.app_category_rules, &app_name);
            AppCategoryOverviewItem {
                app_name: app_name.clone(),
                category: override_category.unwrap_or(item.category),
                total_duration: item.total_duration,
                is_overridden: crate::monitor::find_category_override(
                    &state.config.app_category_rules,
                    &app_name,
                )
                .is_some(),
            }
        })
        .collect())
}

fn upsert_app_category_rule(config: &mut AppConfig, app_name: &str, category: &str) {
    let normalized_app_name = crate::monitor::normalize_display_app_name(app_name);
    let normalized_category = crate::monitor::normalize_category_key(category);
    let match_key = normalized_app_name.to_lowercase();

    if let Some(rule) = config.app_category_rules.iter_mut().find(|rule| {
        crate::monitor::normalize_display_app_name(&rule.app_name).to_lowercase() == match_key
    }) {
        rule.app_name = normalized_app_name;
        rule.category = normalized_category;
        return;
    }

    config.app_category_rules.push(AppCategoryRule {
        app_name: normalized_app_name,
        category: normalized_category,
    });
}

fn reclassify_app_history_in_state(
    state: &AppState,
    app_name: &str,
    category: &str,
) -> Result<usize, AppError> {
    let target_category = crate::monitor::normalize_category_key(category);
    let activities = state
        .database
        .get_activities_by_normalized_app_name(app_name)?;

    for activity in &activities {
        let classification = crate::activity_classifier::classify_activity_with_base_category(
            &activity.app_name,
            &activity.window_title,
            activity.browser_url.as_deref(),
            &target_category,
        );
        state.database.update_activity_classification(
            activity.id.expect("活动记录应包含主键"),
            &classification.base_category,
            Some(&classification.semantic_category),
            Some(i32::from(classification.confidence)),
        )?;
    }

    Ok(activities.len())
}

#[tauri::command]
pub async fn set_app_category_rule(
    app_name: String,
    category: String,
    sync_history: bool,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let trimmed_app_name = app_name.trim();
    if trimmed_app_name.is_empty() {
        return Err(AppError::Unknown("应用名称不能为空".to_string()));
    }

    let next_config = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let mut next_config = state.config.clone();
        upsert_app_category_rule(&mut next_config, trimmed_app_name, &category);
        next_config
    };

    persist_app_config(next_config, app, state.inner())?;

    if !sync_history {
        return Ok(0);
    }

    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    reclassify_app_history_in_state(&state, trimmed_app_name, &category)
}

#[tauri::command]
pub async fn reclassify_app_history(
    app_name: String,
    category: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<usize, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    reclassify_app_history_in_state(&state, &app_name, &category)
}

/// 获取当前运行的应用列表
#[tauri::command]
pub async fn get_running_apps() -> Result<Vec<String>, AppError> {
    get_running_apps_impl()
}

/// macOS 实现
#[cfg(target_os = "macos")]
fn get_running_apps_impl() -> Result<Vec<String>, AppError> {
    use std::process::Command;

    // 使用 AppleScript 获取运行中的应用
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to get name of every process whose background only is false"#
        ])
        .output()
        .map_err(|e| AppError::Unknown(format!("执行 AppleScript 失败: {e}")))?;

    if output.status.success() {
        let apps_str = String::from_utf8_lossy(&output.stdout);
        let mut apps: Vec<String> = apps_str
            .split(", ")
            .map(|s| crate::monitor::normalize_display_app_name(s))
            .filter(|s| !s.is_empty())
            .collect();
        apps.sort();
        apps.dedup();
        Ok(apps)
    } else {
        Err(AppError::Unknown("获取应用列表失败".to_string()))
    }
}

/// Windows 实现
#[cfg(target_os = "windows")]
fn get_running_apps_impl() -> Result<Vec<String>, AppError> {
    use std::collections::HashSet;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::tlhelp32::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut apps = HashSet::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() {
            return Ok(vec![]);
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                // 获取进程名
                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = OsString::from_wide(&entry.szExeFile[..name_len])
                    .to_string_lossy()
                    .to_string();

                // 排除系统进程
                let name_lower = name.to_lowercase();
                if !name_lower.ends_with(".exe") {
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                    continue;
                }

                // 排除常见系统进程
                let excluded = [
                    "svchost.exe",
                    "csrss.exe",
                    "wininit.exe",
                    "services.exe",
                    "lsass.exe",
                    "smss.exe",
                    "winlogon.exe",
                    "dwm.exe",
                    "fontdrvhost.exe",
                    "sihost.exe",
                    "taskhostw.exe",
                    "runtimebroker.exe",
                    "searchhost.exe",
                    "startmenuexperiencehost.exe",
                    "textinputhost.exe",
                    "ctfmon.exe",
                    "conhost.exe",
                ];

                if !excluded.contains(&name_lower.as_str()) {
                    // 移除 .exe 后缀
                    let display_name = crate::monitor::normalize_display_app_name(&name);
                    apps.insert(display_name);
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    let mut result: Vec<String> = apps.into_iter().collect();
    result.sort();
    Ok(result)
}

/// 其他平台
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn get_running_apps_impl() -> Result<Vec<String>, AppError> {
    Ok(vec![])
}

/// 获取存储统计信息
#[tauri::command]
pub async fn get_storage_stats(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let stats = state
        .storage_manager
        .get_stats()
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    Ok(serde_json::json!({
        "total_files": stats.total_files,
        "total_size_mb": format!("{:.1}", stats.total_size_mb),
        "storage_limit_mb": stats.storage_limit_mb,
        "retention_days": stats.retention_days,
    }))
}

/// 获取指定日期的小时摘要
#[tauri::command]
pub async fn get_hourly_summaries(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let summaries = state.database.get_hourly_summaries(&date)?;

    let result: Vec<serde_json::Value> = summaries
        .iter()
        .map(|s| {
            serde_json::json!({
                "hour": s.hour,
                "summary": s.summary,
                "main_apps": s.main_apps,
                "activity_count": s.activity_count,
                "total_duration": s.total_duration,
            })
        })
        .collect();

    Ok(result)
}

/// 清理今天之前的所有活动记录
#[tauri::command]
pub async fn clear_old_activities(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.data_dir.clone()
    };

    // 获取要保留的日期（今天和昨天）
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let yesterday = (now - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let mut deleted_screenshots = 0;

    // 删除旧截图目录（保留今天和昨天）
    let screenshots_dir = data_dir.join("screenshots");
    if screenshots_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&screenshots_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // 保留今天和昨天的目录
                    if name != today && name != yesterday && entry.path().is_dir() {
                        if let Ok(dir_entries) = std::fs::read_dir(entry.path()) {
                            deleted_screenshots += dir_entries.count();
                        }
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }

    // 注意：不删除数据库记录，只删除截图文件
    // OCR 文本保留在数据库中供日报分析使用

    Ok(serde_json::json!({
        "deleted_screenshots": deleted_screenshots,
        "kept_dates": [today, yesterday],
        "message": format!("已清理 {} 张旧截图，保留今天和昨天的数据", deleted_screenshots)
    }))
}

/// 获取指定日期的 OCR 日志
#[tauri::command]
pub async fn get_ocr_log(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let ocr_logger = crate::ocr_logger::OcrLogger::new(&state.data_dir);
    ocr_logger.read_log(&date)
}

/// 检查屏幕锁定状态
#[tauri::command]
pub async fn is_screen_locked() -> Result<bool, AppError> {
    let monitor = crate::screen_lock::ScreenLockMonitor::new();
    Ok(monitor.is_locked())
}

/// 检查 macOS 系统权限状态（屏幕录制 + 辅助功能）
/// Windows 上始终返回全部已授权
#[tauri::command]
pub async fn check_permissions() -> Result<serde_json::Value, AppError> {
    let screen_capture = crate::screenshot::has_screen_capture_permission();
    let accessibility = crate::screenshot::has_accessibility_permission(false);

    Ok(serde_json::json!({
        "screen_capture": screen_capture,
        "accessibility": accessibility,
        "all_granted": screen_capture && accessibility,
        "platform": std::env::consts::OS,
    }))
}

/// 检查是否在工作时间内
#[tauri::command]
pub async fn is_work_time(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, AppError> {
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(crate::screen_lock::ScreenLockMonitor::is_work_time(
        state.config.work_start_hour,
        state.config.work_start_minute,
        state.config.work_end_hour,
        state.config.work_end_minute,
    ))
}

/// 检查 PaddleOCR 是否可用
#[tauri::command]
pub async fn check_ocr_available() -> Result<serde_json::Value, AppError> {
    let paddle_available = crate::ocr::OcrService::check_paddle_available();

    Ok(serde_json::json!({
        "paddle_ocr_available": paddle_available,
        "install_command": crate::ocr::OcrService::get_paddle_install_command(),
        "platform": std::env::consts::OS,
    }))
}

/// 执行 OCR 识别
#[tauri::command]
pub async fn run_ocr(
    screenshot_path: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<serde_json::Value, AppError> {
    let (data_dir, full_path) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let full_path = state.data_dir.join(&screenshot_path);
        (state.data_dir.clone(), full_path)
    };

    if !full_path.exists() {
        return Err(AppError::Unknown(format!("截图文件不存在: {full_path:?}")));
    }

    let ocr_service = crate::ocr::OcrService::new(&data_dir);

    match ocr_service.extract_text(&full_path) {
        Ok(Some(result)) => {
            // 过滤敏感信息
            let filtered_text = crate::ocr::filter_sensitive_text(&result.text);

            Ok(serde_json::json!({
                "success": true,
                "text": filtered_text,
                "raw_text": result.text,
                "confidence": result.confidence,
                "box_count": result.boxes.len(),
            }))
        }
        Ok(None) => Ok(serde_json::json!({
            "success": true,
            "text": "",
            "message": "未检测到文字",
        })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e.to_string(),
        })),
    }
}

/// 获取 OCR 安装指南
#[tauri::command]
pub async fn get_ocr_install_guide() -> Result<serde_json::Value, AppError> {
    let platform = std::env::consts::OS;

    let guide = match platform {
        "windows" => serde_json::json!({
            "platform": "Windows",
            "steps": [
                "1. 确保已安装 Python 3.8+",
                "2. 打开命令提示符或 PowerShell",
                "3. 运行以下命令安装 PaddleOCR：",
                "   pip install paddlepaddle paddleocr -i https://mirror.baidu.com/pypi/simple",
                "4. 等待安装完成（首次运行会自动下载模型）",
                "",
                "备选方案：使用 Windows 内置 OCR（无需安装，但识别效果较弱）"
            ],
            "install_command": "pip install paddlepaddle paddleocr -i https://mirror.baidu.com/pypi/simple",
            "has_builtin_fallback": true,
        }),
        "macos" => serde_json::json!({
            "platform": "macOS",
            "steps": [
                "macOS 使用系统内置的 Vision 框架进行 OCR，无需额外安装。",
                "",
                "如需使用 PaddleOCR（效果更好）：",
                "1. 确保已安装 Python 3.8+",
                "2. 运行以下命令：",
                "   pip install paddlepaddle paddleocr",
            ],
            "install_command": "pip install paddlepaddle paddleocr",
            "has_builtin_fallback": true,
        }),
        _ => serde_json::json!({
            "platform": platform,
            "steps": [
                "1. 确保已安装 Python 3.8+",
                "2. 运行以下命令安装 PaddleOCR：",
                "   pip install paddlepaddle paddleocr",
            ],
            "install_command": "pip install paddlepaddle paddleocr",
            "has_builtin_fallback": false,
        }),
    };

    Ok(guide)
}

/// 设置 Dock 图标可见性 (仅 macOS)
#[tauri::command]
#[allow(unused_variables)]
#[allow(unexpected_cfgs)]
pub fn set_dock_visibility(visible: bool) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        apply_dock_visibility(visible, true);
        log::info!("Dock 图标可见性已设置为: {visible}");
    }

    #[cfg(not(target_os = "macos"))]
    {
        log::warn!("set_dock_visibility 仅支持 macOS");
    }

    Ok(())
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn refresh_dock_icon(activate: bool) {
    use cocoa::appkit::{NSApp, NSImage};
    use cocoa::base::nil;
    use cocoa::foundation::NSString;
    use objc::runtime::Object;

    unsafe {
        let app: *mut Object = NSApp();

        // 使用 NSBundle.mainBundle 获取图标路径
        let bundle: *mut Object = objc::msg_send![objc::class!(NSBundle), mainBundle];
        let resource: *mut Object = objc::msg_send![
            bundle,
            pathForResource: NSString::alloc(nil).init_str("icon")
            ofType: NSString::alloc(nil).init_str("icns")
        ];

        // 如果 bundle 中找不到，尝试硬编码路径
        let path_to_use = if resource != nil {
            resource
        } else {
            NSString::alloc(nil)
                .init_str("/Applications/Work Review.app/Contents/Resources/icon.icns")
        };

        let image: *mut Object = NSImage::alloc(nil).initByReferencingFile_(path_to_use);
        if image != nil {
            let _: () = objc::msg_send![app, setApplicationIconImage: image];
            log::info!("已重新设置 Dock 图标");
        }

        if activate {
            let _: () = objc::msg_send![app, activateIgnoringOtherApps: true];
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn apply_dock_visibility(visible: bool, activate: bool) {
    use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
    use objc::runtime::Object;

    unsafe {
        let app: *mut Object = NSApp();

        if visible {
            // 显示 Dock 图标: 切换回 Regular 策略
            app.setActivationPolicy_(
                NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
            );

            // 切换 ActivationPolicy 后主动重载图标，避免启动后 Dock 残留旧图标缓存
            refresh_dock_icon(activate);
        } else {
            // 隐藏 Dock 图标: 切换到 Accessory 策略
            app.setActivationPolicy_(
                NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn apply_dock_visibility(_visible: bool, _activate: bool) {}

/// 获取应用图标（Base64 PNG）
/// 返回应用的图标，如果获取失败返回空字符串
#[tauri::command]
pub async fn get_app_icon(
    app_name: String,
    executable_path: Option<String>,
) -> Result<String, AppError> {
    get_app_icon_impl(&app_name, executable_path.as_deref()).await
}

#[cfg(any(target_os = "macos", test))]
fn normalize_macos_app_lookup_name(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(".app");
    let mut normalized = String::new();
    let mut last_was_space = false;

    for ch in trimmed.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_alphanumeric() {
            normalized.push(ch);
            last_was_space = false;
        } else if !last_was_space {
            normalized.push(' ');
            last_was_space = true;
        }
    }

    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(any(target_os = "macos", test))]
fn macos_significant_name_tokens(value: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &["app", "browser", "desktop", "helper", "tools"];

    let mut tokens = Vec::new();
    for token in normalize_macos_app_lookup_name(value).split_whitespace() {
        if token.len() < 2 || STOPWORDS.contains(&token) {
            continue;
        }
        if !tokens.iter().any(|existing| existing == token) {
            tokens.push(token.to_string());
        }
    }
    tokens
}

#[cfg(any(target_os = "macos", test))]
fn macos_bundle_path_from_executable(executable_path: &str) -> Option<PathBuf> {
    let path = Path::new(executable_path);
    for ancestor in path.ancestors() {
        let is_app_bundle = ancestor
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("app"))
            .unwrap_or(false);
        if is_app_bundle {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

#[cfg(any(target_os = "macos", test))]
fn macos_score_app_bundle_name(app_name: &str, bundle_name: &str) -> i32 {
    let normalized_app = normalize_macos_app_lookup_name(app_name);
    let normalized_bundle = normalize_macos_app_lookup_name(bundle_name);
    if normalized_app.is_empty() || normalized_bundle.is_empty() {
        return 0;
    }

    let mut score = 0;
    if normalized_app == normalized_bundle {
        score += 1000;
    } else if normalized_app.contains(&normalized_bundle)
        || normalized_bundle.contains(&normalized_app)
    {
        score += 500;
    }

    let app_tokens = macos_significant_name_tokens(&normalized_app);
    let bundle_tokens = macos_significant_name_tokens(&normalized_bundle);
    let overlap_count = bundle_tokens
        .iter()
        .filter(|token| app_tokens.iter().any(|candidate| candidate == *token))
        .count() as i32;
    score += overlap_count * 160;

    if let Some(first_token) = app_tokens.first() {
        if normalized_bundle.starts_with(first_token) {
            score += 80;
        }
    }

    score
}

#[cfg(any(target_os = "macos", test))]
fn collect_macos_app_bundles(root: &Path, depth: usize, bundles: &mut Vec<PathBuf>) {
    if depth == 0 || !root.exists() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let is_app_bundle = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("app"))
            .unwrap_or(false);
        if is_app_bundle {
            bundles.push(path);
            continue;
        }

        collect_macos_app_bundles(&path, depth.saturating_sub(1), bundles);
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_icon_app_path_candidates(app_name: &str, executable_path: Option<&str>) -> Vec<String> {
    let mut candidates: Vec<(i32, String)> = Vec::new();

    if let Some(path) = executable_path.and_then(macos_bundle_path_from_executable) {
        candidates.push((i32::MAX, path.to_string_lossy().to_string()));
    }

    let mut search_roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home_dir) = dirs::home_dir() {
        search_roots.push(home_dir.join("Applications"));
    }

    let mut bundles = Vec::new();
    for root in search_roots {
        collect_macos_app_bundles(&root, 3, &mut bundles);
    }

    for bundle in bundles {
        let Some(bundle_name) = bundle.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let score = macos_score_app_bundle_name(app_name, bundle_name);
        if score <= 0 {
            continue;
        }
        candidates.push((score, bundle.to_string_lossy().to_string()));
    }

    candidates.sort_by(|(score_a, path_a), (score_b, path_b)| {
        score_b
            .cmp(score_a)
            .then_with(|| path_a.len().cmp(&path_b.len()))
            .then_with(|| path_a.cmp(path_b))
    });

    let mut deduped = Vec::new();
    for (_, path) in candidates {
        if deduped.iter().any(|existing| existing == &path) {
            continue;
        }
        deduped.push(path);
    }
    deduped
}

/// macOS 实现：使用 mdfind 获取应用图标（带磁盘缓存）
#[cfg(target_os = "macos")]
async fn get_app_icon_impl(
    app_name: &str,
    executable_path: Option<&str>,
) -> Result<String, AppError> {
    use std::path::Path;
    use std::process::Command;

    // 缓存目录：/tmp/work_review_icons/
    let cache_dir = Path::new("/tmp/work_review_icons");
    if !cache_dir.exists() {
        let _ = std::fs::create_dir_all(cache_dir);
    }

    // 安全文件名：将空格和特殊字符替换为下划线
    let safe_name: String = app_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let cache_file = cache_dir.join(format!("{safe_name}.b64"));

    // 检查缓存：如果缓存存在且非空，直接返回
    if cache_file.exists() {
        if let Ok(cached) = std::fs::read_to_string(&cache_file) {
            if !cached.is_empty() {
                log::debug!("从缓存读取图标: {app_name}");
                return Ok(cached);
            }
        }
    }

    let app_path = macos_icon_app_path_candidates(app_name, executable_path)
        .into_iter()
        .find(|candidate| Path::new(candidate).exists())
        .unwrap_or_default();

    if app_path.is_empty() {
        log::debug!("未找到应用路径: {app_name}");
        return Ok(String::new());
    }

    log::debug!("找到应用路径: {app_name} -> {app_path}");

    // 获取 Info.plist 中的图标文件名
    let info_plist = format!("{app_path}/Contents/Info.plist");
    let icon_name = if Path::new(&info_plist).exists() {
        // 使用 defaults read 读取 CFBundleIconFile
        let defaults_output = Command::new("defaults")
            .args(["read", &info_plist, "CFBundleIconFile"])
            .output();

        if let Ok(output) = defaults_output {
            if output.status.success() {
                let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // 确保有 .icns 扩展名
                if name.ends_with(".icns") {
                    name
                } else {
                    format!("{name}.icns")
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // 构造图标文件路径
    let icns_path = if !icon_name.is_empty() {
        format!("{app_path}/Contents/Resources/{icon_name}")
    } else {
        // 尝试查找任何 .icns 文件
        let find_output = Command::new("find")
            .args([
                &format!("{app_path}/Contents/Resources"),
                "-name",
                "*.icns",
                "-maxdepth",
                "1",
            ])
            .output()
            .map_err(|e| AppError::Unknown(format!("查找图标失败: {e}")))?;

        String::from_utf8_lossy(&find_output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    };

    if icns_path.is_empty() || !Path::new(&icns_path).exists() {
        log::debug!("未找到图标文件: {app_name}");
        return Ok(String::new());
    }

    log::debug!("找到图标文件: {icns_path}");

    // 使用 sips 转换为 PNG
    let temp_png = format!(
        "/tmp/app_icon_{}_{}.png",
        app_name.replace(' ', "_"),
        std::process::id()
    );

    let sips_output = Command::new("sips")
        .args([
            "-s", "format", "png", "-Z", "128", &icns_path, "--out", &temp_png,
        ])
        .output();

    if let Ok(result) = sips_output {
        if result.status.success() {
            if let Ok(png_data) = std::fs::read(&temp_png) {
                let _ = std::fs::remove_file(&temp_png);
                let base64_str =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);
                // 保存到缓存
                let _ = std::fs::write(&cache_file, &base64_str);
                log::debug!("图标已缓存: {} ({} bytes)", app_name, base64_str.len());
                return Ok(base64_str);
            }
        } else {
            log::debug!("sips 转换失败: {}", String::from_utf8_lossy(&result.stderr));
        }
    }

    let _ = std::fs::remove_file(&temp_png);
    Ok(String::new())
}

/// Windows 实现：使用 Shell API 获取高清应用图标
/// 优先提取 256x256 (JUMBO) 图标，降级到 48x48 (EXTRALARGE)，最后回退到 32x32
/// 带磁盘缓存，避免重复启动 PowerShell
#[cfg(any(target_os = "windows", test))]
fn sanitize_icon_cache_name(value: &str) -> String {
    let safe_name: String = value
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();

    if safe_name.is_empty() {
        "icon".to_string()
    } else {
        safe_name
    }
}

#[cfg(any(target_os = "windows", test))]
fn build_windows_icon_cache_key(app_name: &str, executable_path: Option<&str>) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let safe_name = sanitize_icon_cache_name(app_name);
    let Some(path) = executable_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return safe_name;
    };

    let mut hasher = DefaultHasher::new();
    path.to_lowercase().hash(&mut hasher);
    format!("{safe_name}_{:016x}", hasher.finish())
}

#[cfg(any(target_os = "windows", test))]
fn merge_windows_icon_lookup_candidates(
    executable_path: Option<&str>,
    known_icon_paths: Vec<String>,
) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut push_candidate = |value: &str| {
        let candidate = value.trim().trim_matches('"').replace('/', "\\");
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };

    if let Some(path) = executable_path {
        push_candidate(path);
    }

    for path in known_icon_paths {
        push_candidate(&path);
    }

    candidates
}

#[cfg(target_os = "windows")]
fn windows_icon_process_candidates(app_name: &str) -> Vec<String> {
    let trimmed = app_name
        .trim()
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .trim();
    let normalized = trimmed.to_lowercase();
    let compact = normalized
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();

    let mut candidates = Vec::new();
    let mut push_candidate = |value: &str| {
        let candidate = value.trim().to_lowercase();
        if !candidate.is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    };

    push_candidate(trimmed);
    push_candidate(&normalized);
    push_candidate(&compact);

    match compact.as_str() {
        "chrome" | "googlechrome" => {
            push_candidate("chrome");
            push_candidate("google chrome");
        }
        "msedge" | "edge" | "microsoftedge" => {
            push_candidate("msedge");
            push_candidate("microsoft edge");
            push_candidate("edge");
        }
        "brave" | "bravebrowser" => {
            push_candidate("brave");
            push_candidate("brave browser");
        }
        "firefox" => push_candidate("firefox"),
        "safari" => push_candidate("safari"),
        "opera" => push_candidate("opera"),
        "vivaldi" => push_candidate("vivaldi"),
        "chromium" => push_candidate("chromium"),
        "arc" => push_candidate("arc"),
        "zen" | "zenbrowser" => {
            push_candidate("zen");
            push_candidate("zen browser");
        }
        "code" | "vscode" | "visualstudiocode" => {
            push_candidate("code");
            push_candidate("vs code");
            push_candidate("visual studio code");
        }
        "cursor" => push_candidate("cursor"),
        "wechat" | "weixin" => {
            push_candidate("wechat");
            push_candidate("weixin");
        }
        "wecom" => {
            push_candidate("wecom");
            push_candidate("wxwork");
        }
        "qq" => push_candidate("qq"),
        "qqbrowser" => push_candidate("qqbrowser"),
        "360se" | "360chrome" => {
            push_candidate("360se");
            push_candidate("360chrome");
        }
        "sogouexplorer" => push_candidate("sogouexplorer"),
        "explorer" | "fileexplorer" => {
            push_candidate("explorer");
            push_candidate("file explorer");
        }
        "windowsterminal" => {
            push_candidate("windowsterminal");
            push_candidate("windows terminal");
        }
        "powershell" | "pwsh" => {
            push_candidate("powershell");
            push_candidate("pwsh");
        }
        "cmd" | "commandprompt" => {
            push_candidate("cmd");
            push_candidate("command prompt");
        }
        "winword" | "microsoftword" => {
            push_candidate("winword");
            push_candidate("word");
            push_candidate("microsoft word");
        }
        "excel" | "microsoftexcel" => {
            push_candidate("excel");
            push_candidate("microsoft excel");
        }
        "powerpnt" | "powerpoint" | "microsoftpowerpoint" => {
            push_candidate("powerpnt");
            push_candidate("powerpoint");
            push_candidate("microsoft powerpoint");
        }
        "outlook" | "microsoftoutlook" => {
            push_candidate("outlook");
            push_candidate("microsoft outlook");
        }
        _ => {}
    }

    candidates
}

#[cfg(target_os = "windows")]
fn windows_known_icon_paths(app_name: &str) -> Vec<String> {
    let trimmed = app_name
        .trim()
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .trim();
    let normalized = trimmed.to_lowercase();
    let compact = normalized
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>();

    let program_files = std::env::var("ProgramFiles").unwrap_or_default();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let app_data = std::env::var("APPDATA").unwrap_or_default();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

    let mut paths = Vec::new();
    let mut push_path = |path: String| {
        if !path.is_empty() && !paths.contains(&path) {
            paths.push(path);
        }
    };

    match compact.as_str() {
        "explorer" | "fileexplorer" => {
            push_path(format!(r"{}\explorer.exe", windir));
        }
        "msedge" | "edge" | "microsoftedge" => {
            push_path(format!(
                r"{}\Microsoft\Edge\Application\msedge.exe",
                program_files_x86
            ));
            push_path(format!(
                r"{}\Microsoft\Edge\Application\msedge.exe",
                program_files
            ));
        }
        "chrome" | "googlechrome" => {
            push_path(format!(
                r"{}\Google\Chrome\Application\chrome.exe",
                program_files
            ));
            push_path(format!(
                r"{}\Google\Chrome\Application\chrome.exe",
                program_files_x86
            ));
        }
        "wechat" | "weixin" => {
            push_path(format!(r"{}\Tencent\WeChat\WeChat.exe", program_files_x86));
            push_path(format!(r"{}\Tencent\WeChat\WeChat.exe", program_files));
        }
        "wecom" | "wxwork" => {
            push_path(format!(r"{}\Tencent\WeCom\WXWork.exe", program_files_x86));
            push_path(format!(r"{}\Tencent\WeCom\WXWork.exe", program_files));
        }
        "obsidian" => {
            push_path(format!(
                r"{}\Programs\Obsidian\Obsidian.exe",
                local_app_data
            ));
        }
        "pixpin" => {
            push_path(format!(r"{}\PixPin\PixPin.exe", local_app_data));
        }
        "xshell" => {
            push_path(format!(
                r"{}\NetSarang Computer\7\Xshell.exe",
                program_files_x86
            ));
            push_path(format!(
                r"{}\NetSarang Computer\7\Xshell.exe",
                program_files
            ));
            push_path(format!(
                r"{}\NetSarang Computer\8\Xshell.exe",
                program_files_x86
            ));
            push_path(format!(
                r"{}\NetSarang Computer\8\Xshell.exe",
                program_files
            ));
        }
        "wechatappex" => {
            push_path(format!(
                r"{}\Tencent\WeChat\XPlugin\Plugins\WeChatAppEx\WeChatAppEx.exe",
                app_data
            ));
        }
        _ => {}
    }

    paths
}

#[cfg(target_os = "windows")]
fn encode_windows_icon_path(value: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
unsafe fn get_windows_icon_from_shell_image_list(
    path: &str,
    list_type: i32,
) -> Option<winapi::shared::windef::HICON> {
    use std::mem::zeroed;
    use std::ptr::null_mut;
    use winapi::ctypes::c_void;
    use winapi::um::commoncontrols::IImageList;
    use winapi::um::shellapi::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX};
    use winapi::Interface;

    let wide_path = encode_windows_icon_path(path);
    let mut file_info: SHFILEINFOW = zeroed();
    let lookup_result = SHGetFileInfoW(
        wide_path.as_ptr(),
        0,
        &mut file_info,
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_SYSICONINDEX,
    );
    if lookup_result == 0 {
        return None;
    }

    let mut image_list: *mut IImageList = null_mut();
    let hr = SHGetImageList(
        list_type,
        &IImageList::uuidof(),
        &mut image_list as *mut _ as *mut *mut c_void,
    );
    if hr < 0 || image_list.is_null() {
        return None;
    }

    let mut icon = null_mut();
    let hr = (*image_list).GetIcon(file_info.iIcon, 0, &mut icon);
    (*image_list).Release();

    if hr < 0 || icon.is_null() {
        None
    } else {
        Some(icon)
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_windows_associated_icon(path: &str) -> Option<winapi::shared::windef::HICON> {
    use std::ptr::null_mut;
    use winapi::shared::minwindef::WORD;
    use winapi::um::shellapi::ExtractAssociatedIconW;

    let mut wide_path = encode_windows_icon_path(path);
    if wide_path.len() < 260 {
        wide_path.resize(260, 0);
    }

    let mut icon_index: WORD = 0;
    let icon = ExtractAssociatedIconW(null_mut(), wide_path.as_mut_ptr(), &mut icon_index);
    if icon.is_null() {
        None
    } else {
        Some(icon)
    }
}

#[cfg(target_os = "windows")]
unsafe fn render_windows_icon_pixels(
    icon: winapi::shared::windef::HICON,
) -> Option<(Vec<u8>, u32, u32)> {
    const DI_NORMAL: u32 = 0x0003;

    use std::mem::zeroed;
    use std::ptr::{copy_nonoverlapping, null_mut, write_bytes};
    use winapi::shared::minwindef::UINT;
    use winapi::shared::windef::HGDIOBJ;
    use winapi::um::wingdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetObjectW, SelectObject,
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use winapi::um::winuser::{DrawIconEx, GetDC, GetIconInfo, ReleaseDC, ICONINFO};

    let mut icon_info: ICONINFO = zeroed();
    if GetIconInfo(icon, &mut icon_info) == 0 {
        return None;
    }

    let rendered = (|| {
        let source_bitmap = if !icon_info.hbmColor.is_null() {
            icon_info.hbmColor
        } else {
            icon_info.hbmMask
        };
        if source_bitmap.is_null() {
            return None;
        }

        let mut bitmap: BITMAP = zeroed();
        let get_object_result = GetObjectW(
            source_bitmap as *mut _,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bitmap as *mut _ as *mut _,
        );
        if get_object_result == 0 {
            return None;
        }

        let width = bitmap.bmWidth.abs();
        let mut height = bitmap.bmHeight.abs();
        if icon_info.hbmColor.is_null() {
            height /= 2;
        }
        if width <= 0 || height <= 0 {
            return None;
        }

        let screen_dc = GetDC(null_mut());
        if screen_dc.is_null() {
            return None;
        }

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(null_mut(), screen_dc);
            return None;
        }

        let mut bitmap_info: BITMAPINFO = zeroed();
        bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = width;
        bitmap_info.bmiHeader.biHeight = -height;
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = BI_RGB;

        let mut dib_bits = null_mut();
        let dib = CreateDIBSection(
            screen_dc,
            &bitmap_info,
            DIB_RGB_COLORS as UINT,
            &mut dib_bits,
            null_mut(),
            0,
        );
        if dib.is_null() || dib_bits.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);
            return None;
        }

        let old_object = SelectObject(mem_dc, dib as HGDIOBJ);
        if old_object.is_null() {
            DeleteObject(dib as HGDIOBJ);
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);
            return None;
        }

        let pixel_len = width as usize * height as usize * 4;
        write_bytes(dib_bits as *mut u8, 0, pixel_len);

        let draw_result = DrawIconEx(mem_dc, 0, 0, icon, width, height, 0, null_mut(), DI_NORMAL);
        let mut pixels = None;
        if draw_result != 0 {
            let mut buffer = vec![0; pixel_len];
            copy_nonoverlapping(dib_bits as *const u8, buffer.as_mut_ptr(), pixel_len);
            pixels = Some((buffer, width as u32, height as u32));
        }

        SelectObject(mem_dc, old_object);
        DeleteObject(dib as HGDIOBJ);
        DeleteDC(mem_dc);
        ReleaseDC(null_mut(), screen_dc);
        pixels
    })();

    if !icon_info.hbmColor.is_null() {
        DeleteObject(icon_info.hbmColor as HGDIOBJ);
    }
    if !icon_info.hbmMask.is_null() {
        DeleteObject(icon_info.hbmMask as HGDIOBJ);
    }

    rendered
}

#[cfg(target_os = "windows")]
fn encode_windows_icon_base64(mut pixels: Vec<u8>, width: u32, height: u32) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }

    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    let image = image::RgbaImage::from_raw(width, height, pixels)?;
    let mut dynamic_image = image::DynamicImage::ImageRgba8(image);
    if width > 128 || height > 128 {
        dynamic_image = dynamic_image.resize_exact(128, 128, image::imageops::FilterType::Lanczos3);
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    dynamic_image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;

    Some(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        cursor.into_inner(),
    ))
}

#[cfg(target_os = "windows")]
fn convert_windows_icon_to_base64(icon: winapi::shared::windef::HICON) -> Option<(String, u32)> {
    use winapi::um::winuser::DestroyIcon;

    let rendered = unsafe { render_windows_icon_pixels(icon) };
    unsafe {
        DestroyIcon(icon);
    }

    let (pixels, width, height) = rendered?;
    let encoded = encode_windows_icon_base64(pixels, width, height)?;
    Some((encoded, width.max(height)))
}

#[cfg(target_os = "windows")]
fn extract_windows_icon_base64(path: &str) -> Option<String> {
    use winapi::um::shellapi::{SHIL_EXTRALARGE, SHIL_JUMBO};

    let mut jumbo_fallback = None;

    if let Some(icon) = unsafe { get_windows_icon_from_shell_image_list(path, SHIL_JUMBO as i32) } {
        if let Some((encoded, size)) = convert_windows_icon_to_base64(icon) {
            if size >= 48 {
                return Some(encoded);
            }
            jumbo_fallback = Some(encoded);
        }
    }

    let mut extra_large_fallback = None;
    if let Some(icon) =
        unsafe { get_windows_icon_from_shell_image_list(path, SHIL_EXTRALARGE as i32) }
    {
        if let Some((encoded, size)) = convert_windows_icon_to_base64(icon) {
            if size >= 32 {
                return Some(encoded);
            }
            extra_large_fallback = Some(encoded);
        }
    }

    if let Some(icon) = unsafe { get_windows_associated_icon(path) } {
        if let Some((encoded, _)) = convert_windows_icon_to_base64(icon) {
            return Some(encoded);
        }
    }

    extra_large_fallback.or(jumbo_fallback)
}

#[cfg(target_os = "windows")]
async fn get_app_icon_impl(
    app_name: &str,
    executable_path: Option<&str>,
) -> Result<String, AppError> {
    const WINDOWS_ICON_CACHE_VERSION: &str = "v5";

    // 磁盘缓存：检查是否已有缓存
    let cache_dir = std::env::temp_dir().join("work_review_icons");
    let _ = std::fs::create_dir_all(&cache_dir);
    let cache_key = build_windows_icon_cache_key(
        &crate::monitor::normalize_display_app_name(app_name),
        executable_path,
    );
    let cache_file = cache_dir.join(format!("{cache_key}_{WINDOWS_ICON_CACHE_VERSION}.b64"));

    if cache_file.exists() {
        if let Ok(metadata) = std::fs::metadata(&cache_file) {
            // 缓存有效期 24 小时
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or_default().as_secs() < 86400 {
                    if let Ok(cached) = std::fs::read_to_string(&cache_file) {
                        if cached.len() > 100 {
                            return Ok(cached);
                        }
                    }
                }
            }
        }
    }

    let icon_lookup_candidates = merge_windows_icon_lookup_candidates(
        executable_path,
        windows_known_icon_paths(app_name)
            .into_iter()
            .filter(|path| std::path::Path::new(path).exists())
            .collect::<Vec<_>>(),
    );
    if icon_lookup_candidates.is_empty() {
        return Ok(String::new());
    }

    // 仅对明确的可执行路径提取图标，不扫描注册表、开始菜单快捷方式或全部运行进程。
    for candidate_path in icon_lookup_candidates {
        if !Path::new(&candidate_path).exists() {
            continue;
        }

        if let Some(base64_str) = extract_windows_icon_base64(&candidate_path) {
            if base64_str.len() > 100 {
                let _ = std::fs::write(&cache_file, &base64_str);
                return Ok(base64_str);
            }
        }
    }

    Ok(String::new())
}

/// 其他平台：返回空字符串
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn get_app_icon_impl(
    _app_name: &str,
    _executable_path: Option<&str>,
) -> Result<String, AppError> {
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::{
        build_fallback_assistant_answer, build_updater_manifest_candidates,
        build_windows_icon_cache_key, detect_assistant_question_kind,
        detect_assistant_question_kind_with_mode, format_browser_url_for_display,
        macos_score_app_bundle_name, merge_windows_icon_lookup_candidates,
        normalize_macos_app_lookup_name, AssistantChatMessage, AssistantQuestionKind,
        AssistantReasoningMode,
    };
    use crate::database::MemorySearchItem;
    use crate::work_intelligence::{
        IntentAnalysisResult, IntentSummary, NamedDuration, WeeklyReviewResult, WorkSession,
    };

    fn sample_review() -> WeeklyReviewResult {
        WeeklyReviewResult {
            title: "阶段复盘".to_string(),
            markdown: "本周主要推进了助手回答链路改造。".to_string(),
            total_duration: 6 * 3600,
            active_days: 3,
            session_count: 5,
            deep_work_sessions: 2,
            top_intents: vec![IntentSummary {
                label: "编码开发".to_string(),
                duration: 4 * 3600,
                session_count: 3,
            }],
            top_apps: vec![NamedDuration {
                name: "VS Code".to_string(),
                duration: 4 * 3600,
            }],
            highlights: vec!["完成助手回答链路重构设计".to_string()],
            risks: vec!["追问场景仍依赖关键词".to_string()],
        }
    }

    fn sample_intents() -> IntentAnalysisResult {
        IntentAnalysisResult {
            sessions: vec![],
            summary: vec![IntentSummary {
                label: "编码开发".to_string(),
                duration: 4 * 3600,
                session_count: 3,
            }],
        }
    }

    fn sample_sessions() -> Vec<WorkSession> {
        vec![WorkSession {
            session_id: "2026-03-27-1".to_string(),
            date: "2026-03-27".to_string(),
            start_timestamp: 1_711_500_000,
            end_timestamp: 1_711_503_600,
            duration: 3600,
            activity_count: 4,
            app_count: 2,
            dominant_app: "VS Code".to_string(),
            dominant_category: "development".to_string(),
            title: "重构助手回答链路".to_string(),
            browser_domains: vec!["github.com".to_string()],
            top_apps: vec![NamedDuration {
                name: "VS Code".to_string(),
                duration: 3000,
            }],
            top_keywords: vec!["assistant".to_string()],
            intent_label: "编码开发".to_string(),
            intent_confidence: 88,
            intent_evidence: vec!["修改 commands.rs".to_string()],
        }]
    }

    fn sample_references() -> Vec<MemorySearchItem> {
        vec![MemorySearchItem {
            source_type: "activity".to_string(),
            source_id: Some(1),
            date: "2026-03-27".to_string(),
            timestamp: 1_711_503_600,
            title: "重构助手回答链路".to_string(),
            excerpt: "完成问题分类和回答骨架调整".to_string(),
            app_name: Some("VS Code".to_string()),
            browser_url: None,
            duration: Some(3600),
            score: 120,
        }]
    }

    #[test]
    fn 应将命令输出中的_url_格式化为可读文本() {
        assert_eq!(
            format_browser_url_for_display(
                "https://www.google.com.hk/search?q=%E5%A4%A7%E6%B8%A1%E5%8F%A3&client=firefox-b-d"
            ),
            "https://www.google.com.hk/search?q=大渡口&client=firefox-b-d"
        );
        assert_eq!(
            format_browser_url_for_display(
                "https://example.com/search?q=a%26b&name=%E5%BC%A0%E4%B8%89"
            ),
            "https://example.com/search?q=a%26b&name=张三"
        );
    }

    fn sample_noisy_references() -> Vec<MemorySearchItem> {
        vec![
            MemorySearchItem {
                source_type: "activity".to_string(),
                source_id: Some(2),
                date: "2026-03-27".to_string(),
                timestamp: 1_711_504_200,
                title: "../Pycharm_Project/Work_Review/src-tauri".to_string(),
                excerpt: "编辑 显示 通知 窗口 帮助 Work Review 记录 分析 证明 记录状态 暂停 时间线 日报".to_string(),
                app_name: Some("cmux".to_string()),
                browser_url: None,
                duration: Some(1800),
                score: 100,
            },
            MemorySearchItem {
                source_type: "activity".to_string(),
                source_id: Some(3),
                date: "2026-03-27".to_string(),
                timestamp: 1_711_504_500,
                title: "无标题 - Google Chrome - momoi".to_string(),
                excerpt: "Chrome 文件 编辑 显示 历史记录 书签 个人资料 标签页 窗口 帮助 Work Review 记录 分析".to_string(),
                app_name: Some("Google Chrome".to_string()),
                browser_url: None,
                duration: Some(900),
                score: 96,
            },
        ]
    }

    fn sample_process_follow_up_history() -> Vec<AssistantChatMessage> {
        vec![
            AssistantChatMessage {
                role: "user".to_string(),
                content: "最近时间主要花在哪？".to_string(),
            },
            AssistantChatMessage {
                role: "assistant".to_string(),
                content: "## 结论\n\n- 这段时间更像是围绕少数主题持续推进。\n\n## 过程分析\n\n- 主要是编码开发相关 session。\n".to_string(),
            },
        ]
    }

    fn sample_stage_follow_up_history() -> Vec<AssistantChatMessage> {
        vec![
            AssistantChatMessage {
                role: "user".to_string(),
                content: "这周主要做了什么？".to_string(),
            },
            AssistantChatMessage {
                role: "assistant".to_string(),
                content: "## 结论\n\n- 这周主线是助手回答链路改造。\n".to_string(),
            },
        ]
    }

    #[test]
    fn windows图标候选应优先真实路径并去重() {
        let candidates = merge_windows_icon_lookup_candidates(
            Some(r"D:\Portable\Code\Code.exe"),
            vec![
                r"C:\Program Files\Microsoft VS Code\Code.exe".to_string(),
                r"D:\Portable\Code\Code.exe".to_string(),
                r"C:\Program Files\Microsoft VS Code\Code.exe".to_string(),
            ],
        );

        assert_eq!(
            candidates,
            vec![
                r"D:\Portable\Code\Code.exe".to_string(),
                r"C:\Program Files\Microsoft VS Code\Code.exe".to_string(),
            ]
        );
    }

    #[test]
    fn windows图标缓存key应包含真实路径特征() {
        let portable_key =
            build_windows_icon_cache_key("VS Code", Some(r"D:\Portable\Code\Code.exe"));
        let installed_key = build_windows_icon_cache_key(
            "VS Code",
            Some(r"C:\Program Files\Microsoft VS Code\Code.exe"),
        );

        assert_ne!(portable_key, installed_key);
        assert!(portable_key.starts_with("VS_Code_"));
        assert!(installed_key.starts_with("VS_Code_"));
    }

    #[test]
    fn macos图标名称归一化应兼容分隔符与后缀() {
        assert_eq!(
            normalize_macos_app_lookup_name("Zen Browser"),
            "zen browser"
        );
        assert_eq!(
            normalize_macos_app_lookup_name("antigravity_tools.app"),
            "antigravity tools"
        );
        assert_eq!(
            normalize_macos_app_lookup_name("Antigravity-Tools"),
            "antigravity tools"
        );
    }

    #[test]
    fn macos应用包名评分应兼容缩写与分隔符差异() {
        assert!(
            macos_score_app_bundle_name("Foo Browser", "Foo")
                > macos_score_app_bundle_name("Foo Browser", "Bar")
        );
        assert!(
            macos_score_app_bundle_name("antigravity_tools", "Antigravity")
                > macos_score_app_bundle_name("antigravity_tools", "Calculator")
        );
        assert!(
            macos_score_app_bundle_name("antigravity_tools", "Antigravity Tools")
                >= macos_score_app_bundle_name("antigravity_tools", "Antigravity")
        );
    }

    #[test]
    fn 更新清单候选应优先显式版本地址再回退latest地址() {
        let candidates = build_updater_manifest_candidates(
            "https://github.com/wm94i/Work_Review/releases/latest/download/updater.json",
            Some("1.0.24"),
        );

        assert_eq!(
            candidates,
            vec![
                "https://github.com/wm94i/Work_Review/releases/download/v1.0.24/updater.json"
                    .to_string(),
                "https://github.com/wm94i/Work_Review/releases/latest/download/updater.json"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn 助手问题分类应识别阶段总结与过程复盘和证据追问() {
        assert_eq!(
            detect_assistant_question_kind("这周主要做了什么？", &[]),
            AssistantQuestionKind::StageSummary
        );
        assert_eq!(
            detect_assistant_question_kind("最近时间主要花在哪？", &[]),
            AssistantQuestionKind::ProcessRecap
        );
        assert_eq!(
            detect_assistant_question_kind("这个结论的依据是什么？", &[]),
            AssistantQuestionKind::EvidenceQuery
        );
    }

    #[test]
    fn 更新清单候选应保留代理前缀并规范化版本号() {
        let candidates = build_updater_manifest_candidates(
            "https://ghproxy.cn/https://github.com/wm94i/Work_Review/releases/latest/download/updater-ghproxy.json",
            Some("v1.0.24"),
        );

        assert_eq!(
            candidates,
            vec![
                "https://ghproxy.cn/https://github.com/wm94i/Work_Review/releases/download/v1.0.24/updater-ghproxy.json"
                    .to_string(),
                "https://ghproxy.cn/https://github.com/wm94i/Work_Review/releases/latest/download/updater-ghproxy.json"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn 助手问题分类应继承上一轮过程复盘语境() {
        let history = sample_process_follow_up_history();

        assert_eq!(
            detect_assistant_question_kind("继续", &history),
            AssistantQuestionKind::ProcessRecap
        );
        assert_eq!(
            detect_assistant_question_kind("展开说说这个", &history),
            AssistantQuestionKind::ProcessRecap
        );
    }

    #[test]
    fn 助手问题分类应将依据追问优先识别为证据问题() {
        let history = sample_stage_follow_up_history();

        assert_eq!(
            detect_assistant_question_kind("那依据呢", &history),
            AssistantQuestionKind::EvidenceQuery
        );
        assert_eq!(
            detect_assistant_question_kind("这个结论怎么得出的", &history),
            AssistantQuestionKind::EvidenceQuery
        );
    }

    #[test]
    fn ai增强识别器应比基础模板更强承接助手上下文() {
        let history = vec![
            AssistantChatMessage {
                role: "user".to_string(),
                content: "这周主要做了什么？".to_string(),
            },
            AssistantChatMessage {
                role: "assistant".to_string(),
                content: "## 结论\n\n- 这周主线是助手回答链路改造。\n\n## 过程分析\n\n- 主要是编码开发相关 session。\n".to_string(),
            },
        ];

        assert_eq!(
            detect_assistant_question_kind_with_mode(
                "展开说说这个",
                &history,
                AssistantReasoningMode::Basic
            ),
            AssistantQuestionKind::StageSummary
        );
        assert_eq!(
            detect_assistant_question_kind_with_mode(
                "展开说说这个",
                &history,
                AssistantReasoningMode::AiEnhanced
            ),
            AssistantQuestionKind::ProcessRecap
        );
    }

    #[test]
    fn 复盘型基础回答应输出统一章节骨架() {
        let answer = build_fallback_assistant_answer(
            "这周主要做了什么？",
            AssistantQuestionKind::StageSummary,
            &sample_references(),
            Some(&sample_sessions()),
            Some(&sample_intents()),
            Some(&sample_review()),
            None,
            &["周报复盘".to_string()],
        );

        assert!(answer.contains("## 结论"));
        assert!(answer.contains("## 结果概览"));
        assert!(answer.contains("## 过程分析"));
        assert!(answer.contains("## 依据补充"));
        assert!(answer.contains("## 复盘总结"));
    }

    #[test]
    fn 低信噪比原始记录不应直接出现在依据补充中() {
        let answer = build_fallback_assistant_answer(
            "这周主要做了什么？",
            AssistantQuestionKind::StageSummary,
            &sample_noisy_references(),
            Some(&sample_sessions()),
            Some(&sample_intents()),
            Some(&sample_review()),
            None,
            &["周报复盘".to_string()],
        );

        assert!(!answer.contains("../Pycharm_Project/Work_Review/src-tauri"));
        assert!(!answer.contains("无标题 - Google Chrome - momoi"));
        assert!(answer.contains("直接命中的原始记录区分度不高"));
    }
}

/// 保存背景图片（接收 base64 编码的图片数据）
#[tauri::command]
pub async fn save_background_image(
    data: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let (data_dir, config_path) = {
        let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (s.data_dir.clone(), s.config_path.clone())
    };

    // 解码 base64
    let image_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
        .map_err(|e| AppError::Unknown(format!("base64 解码失败: {e}")))?;

    // 保存为 JPEG（压缩体积）
    let img = image::load_from_memory(&image_bytes)
        .map_err(|e| AppError::Unknown(format!("图片解析失败: {e}")))?;

    // 限制最大尺寸为 1920px 宽
    let img = if img.width() > 1920 {
        img.resize(1920, 1920, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let bg_path = data_dir.join("background.jpg");
    img.save_with_format(&bg_path, image::ImageFormat::Jpeg)
        .map_err(|e| AppError::Unknown(format!("保存背景图失败: {e}")))?;

    // 更新配置
    let mut s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    s.config.background_image = Some("background.jpg".to_string());
    s.config.save(&config_path)?;

    Ok(())
}

/// 获取背景图片（返回 base64）
#[tauri::command]
pub async fn get_background_image(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<String>, AppError> {
    let (data_dir, bg_filename) = {
        let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (s.data_dir.clone(), s.config.background_image.clone())
    };

    let filename = match bg_filename {
        Some(f) if !f.is_empty() => f,
        _ => return Ok(None),
    };

    let bg_path = data_dir.join(&filename);
    if !bg_path.exists() {
        return Ok(None);
    }

    let bytes =
        std::fs::read(&bg_path).map_err(|e| AppError::Unknown(format!("读取背景图失败: {e}")))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok(Some(b64))
}

/// 清除背景图片
#[tauri::command]
pub async fn clear_background_image(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let (data_dir, config_path) = {
        let s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (s.data_dir.clone(), s.config_path.clone())
    };

    // 删除文件
    let bg_path = data_dir.join("background.jpg");
    if bg_path.exists() {
        let _ = std::fs::remove_file(&bg_path);
    }

    // 更新配置
    let mut s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    s.config.background_image = None;
    s.config.save(&config_path)?;

    Ok(())
}
