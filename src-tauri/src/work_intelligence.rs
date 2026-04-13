use crate::analysis::format_duration;
use crate::database::Activity;
use chrono::{Local, TimeZone};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const SESSION_GAP_SECONDS: i64 = 15 * 60;
const DEEP_WORK_THRESHOLD_SECONDS: i64 = 45 * 60;
const TODO_TEXT_LIMIT: usize = 80;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NamedDuration {
    pub name: String,
    pub duration: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentSummary {
    pub label: String,
    pub duration: i64,
    pub session_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkSession {
    pub session_id: String,
    pub date: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub duration: i64,
    pub activity_count: usize,
    pub app_count: usize,
    pub dominant_app: String,
    pub dominant_category: String,
    pub title: String,
    pub browser_domains: Vec<String>,
    pub top_apps: Vec<NamedDuration>,
    pub top_keywords: Vec<String>,
    pub intent_label: String,
    pub intent_confidence: i32,
    pub intent_evidence: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntentAnalysisResult {
    pub sessions: Vec<WorkSession>,
    pub summary: Vec<IntentSummary>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReviewResult {
    pub title: String,
    pub markdown: String,
    pub total_duration: i64,
    pub active_days: usize,
    pub session_count: usize,
    pub deep_work_sessions: usize,
    pub top_intents: Vec<IntentSummary>,
    pub top_apps: Vec<NamedDuration>,
    pub highlights: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub title: String,
    pub date: String,
    pub source_title: String,
    pub source_app: String,
    pub confidence: i32,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TodoExtractionResult {
    pub items: Vec<TodoItem>,
    pub summary: String,
}

#[derive(Debug, Clone)]
struct IntentMatch {
    label: String,
    score: i32,
    evidence: Vec<String>,
}

#[derive(Default)]
struct SessionAccumulator {
    activities: Vec<Activity>,
    start_timestamp: i64,
    end_timestamp: i64,
    total_duration: i64,
}

impl SessionAccumulator {
    fn from_activity(activity: Activity) -> Self {
        let start_timestamp = activity.timestamp;
        let end_timestamp = activity.timestamp + activity.duration.max(1);
        Self {
            activities: vec![activity],
            start_timestamp,
            end_timestamp,
            total_duration: end_timestamp - start_timestamp,
        }
    }

    fn can_merge(&self, next: &Activity) -> bool {
        next.timestamp - self.end_timestamp <= SESSION_GAP_SECONDS
    }

    fn push(&mut self, activity: Activity) {
        self.end_timestamp = self
            .end_timestamp
            .max(activity.timestamp + activity.duration.max(1));
        self.total_duration += activity.duration.max(1);
        self.activities.push(activity);
    }

    fn finalize(self, index: usize) -> WorkSession {
        let mut app_durations: HashMap<String, i64> = HashMap::new();
        let mut category_durations: HashMap<String, i64> = HashMap::new();
        let mut domain_durations: HashMap<String, i64> = HashMap::new();
        let mut keyword_counts: HashMap<String, i32> = HashMap::new();
        let mut title_counts: HashMap<String, i32> = HashMap::new();
        let mut combined_lines = Vec::new();

        for activity in &self.activities {
            *app_durations.entry(activity.app_name.clone()).or_insert(0) +=
                activity.duration.max(1);
            *category_durations
                .entry(activity.category.clone())
                .or_insert(0) += activity.duration.max(1);

            if !activity.window_title.trim().is_empty() {
                *title_counts
                    .entry(activity.window_title.trim().to_string())
                    .or_insert(0) += 1;
            }

            if let Some(url) = &activity.browser_url {
                let domain = extract_domain(url);
                if !domain.is_empty() {
                    *domain_durations.entry(domain).or_insert(0) += activity.duration.max(1);
                }
            }

            let mut text_parts = vec![activity.app_name.as_str(), activity.window_title.as_str()];
            if let Some(ocr_text) = &activity.ocr_text {
                text_parts.push(ocr_text.as_str());
            }
            if let Some(url) = &activity.browser_url {
                text_parts.push(url.as_str());
            }

            let joined = text_parts.join(" ");
            combined_lines.push(joined.clone());
            for token in extract_keywords(&joined) {
                *keyword_counts.entry(token).or_insert(0) += 1;
            }
        }

        let dominant_app =
            top_name_by_duration(&app_durations).unwrap_or_else(|| "未知应用".to_string());
        let dominant_category =
            top_name_by_duration(&category_durations).unwrap_or_else(|| "unknown".to_string());
        let title = top_name_by_count(&title_counts).unwrap_or_else(|| dominant_app.clone());
        let browser_domains = top_named_durations(&domain_durations, 4)
            .into_iter()
            .map(|item| item.name)
            .collect::<Vec<_>>();
        let top_apps = top_named_durations(&app_durations, 4);
        let top_keywords = top_keywords(&keyword_counts, 6);
        let intent = classify_session(
            &dominant_category,
            &dominant_app,
            &title,
            &browser_domains,
            &top_keywords,
            &combined_lines,
        );

        WorkSession {
            session_id: format!(
                "{}-{}-{}",
                date_from_timestamp(self.start_timestamp),
                self.start_timestamp,
                index + 1
            ),
            date: date_from_timestamp(self.start_timestamp),
            start_timestamp: self.start_timestamp,
            end_timestamp: self.end_timestamp,
            duration: self.total_duration.max(1),
            activity_count: self.activities.len(),
            app_count: app_durations.len(),
            dominant_app,
            dominant_category,
            title,
            browser_domains,
            top_apps,
            top_keywords,
            intent_label: intent.label,
            intent_confidence: intent.score.clamp(35, 95),
            intent_evidence: intent.evidence,
        }
    }
}

pub fn build_work_sessions(activities: &[Activity]) -> Vec<WorkSession> {
    let mut sorted = activities.to_vec();
    sorted.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then_with(|| a.id.cmp(&b.id)));

    let mut sessions = Vec::new();
    let mut current: Option<SessionAccumulator> = None;

    for activity in sorted {
        match current.as_mut() {
            Some(acc) if acc.can_merge(&activity) => acc.push(activity),
            Some(acc) => {
                let finished = std::mem::take(acc);
                sessions.push(finished.finalize(sessions.len()));
                *acc = SessionAccumulator::from_activity(activity);
            }
            None => current = Some(SessionAccumulator::from_activity(activity)),
        }
    }

    if let Some(acc) = current {
        sessions.push(acc.finalize(sessions.len()));
    }

    sessions.sort_by(|a, b| b.start_timestamp.cmp(&a.start_timestamp));
    sessions
}

pub fn analyze_intents(activities: &[Activity]) -> IntentAnalysisResult {
    let sessions = build_work_sessions(activities);
    let mut summary_map: HashMap<String, (i64, usize)> = HashMap::new();

    for session in &sessions {
        let entry = summary_map
            .entry(session.intent_label.clone())
            .or_insert((0, 0));
        entry.0 += session.duration;
        entry.1 += 1;
    }

    let mut summary = summary_map
        .into_iter()
        .map(|(label, (duration, session_count))| IntentSummary {
            label,
            duration,
            session_count,
        })
        .collect::<Vec<_>>();

    summary.sort_by(|a, b| {
        b.duration
            .cmp(&a.duration)
            .then_with(|| b.session_count.cmp(&a.session_count))
            .then_with(|| a.label.cmp(&b.label))
    });

    IntentAnalysisResult { sessions, summary }
}

pub fn generate_weekly_review(
    activities: &[Activity],
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> WeeklyReviewResult {
    let analysis = analyze_intents(activities);
    let total_duration = analysis
        .sessions
        .iter()
        .map(|session| session.duration)
        .sum::<i64>();
    let session_count = analysis.sessions.len();
    let deep_work_sessions = analysis
        .sessions
        .iter()
        .filter(|session| session.duration >= DEEP_WORK_THRESHOLD_SECONDS)
        .count();

    let mut day_durations: HashMap<String, i64> = HashMap::new();
    let mut app_durations: HashMap<String, i64> = HashMap::new();
    let mut context_switch_sessions = 0usize;

    for session in &analysis.sessions {
        *day_durations.entry(session.date.clone()).or_insert(0) += session.duration;
        for item in &session.top_apps {
            *app_durations.entry(item.name.clone()).or_insert(0) += item.duration;
        }
        if session.app_count >= 4 {
            context_switch_sessions += 1;
        }
    }

    let active_days = day_durations.len();
    let top_apps = top_named_durations(&app_durations, 5);
    let top_intents = analysis.summary.iter().take(5).cloned().collect::<Vec<_>>();
    let busiest_day = day_durations
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)))
        .map(|(day, duration)| (day.clone(), *duration));

    let average_session_duration = if session_count > 0 {
        total_duration / session_count as i64
    } else {
        0
    };

    let mut highlights = Vec::new();
    if let Some(primary_intent) = top_intents.first() {
        highlights.push(format!(
            "本阶段投入最多的是“{}”，累计 {}。",
            primary_intent.label,
            format_duration(primary_intent.duration)
        ));
    }
    if let Some((day, duration)) = busiest_day.as_ref() {
        highlights.push(format!(
            "{} 是投入最高的一天，累计 {}。",
            day,
            format_duration(*duration)
        ));
    }
    if deep_work_sessions > 0 {
        highlights.push(format!(
            "出现 {} 段超过 45 分钟的连续专注时段。",
            deep_work_sessions
        ));
    }
    if highlights.is_empty() {
        highlights.push("当前时间范围内记录较少，暂时无法形成稳定模式。".to_string());
    }

    let mut risks = Vec::new();
    if average_session_duration > 0 && average_session_duration < 20 * 60 {
        risks.push("平均 session 偏短，任务切换可能较频繁。".to_string());
    }
    if context_switch_sessions > 0
        && session_count > 0
        && context_switch_sessions * 2 >= session_count
    {
        risks.push("多应用混合 session 偏多，建议留意上下文切换成本。".to_string());
    }
    if let Some(meeting) = top_intents.iter().find(|item| item.label == "会议沟通") {
        if meeting.duration * 4 >= total_duration.max(1) {
            risks.push("会议沟通占比较高，注意给深度工作预留整块时间。".to_string());
        }
    }
    if risks.is_empty() {
        risks.push("整体节奏较稳定，没有明显异常信号。".to_string());
    }

    let title = format!(
        "工作复盘（{} ~ {}）",
        date_from.unwrap_or("最早记录"),
        date_to.unwrap_or("最近记录")
    );

    let mut markdown = String::new();
    markdown.push_str(&format!("# {title}\n\n"));
    markdown.push_str("## 本期概览\n\n");
    markdown.push_str(&format!(
        "- 总投入时长：{}\n",
        format_duration(total_duration)
    ));
    markdown.push_str(&format!("- 活跃天数：{} 天\n", active_days));
    markdown.push_str(&format!("- session 数量：{} 段\n", session_count));
    markdown.push_str(&format!("- 深度工作段：{} 段\n\n", deep_work_sessions));

    markdown.push_str("## 重点工作\n\n");
    if top_intents.is_empty() {
        markdown.push_str("- 暂无足够数据识别重点工作。\n");
    } else {
        for item in &top_intents {
            markdown.push_str(&format!(
                "- {}：{}，共 {} 段\n",
                item.label,
                format_duration(item.duration),
                item.session_count
            ));
        }
    }
    markdown.push('\n');

    markdown.push_str("## 核心观察\n\n");
    for line in &highlights {
        markdown.push_str(&format!("- {line}\n"));
    }
    markdown.push('\n');

    markdown.push_str("## 风险与提醒\n\n");
    for line in &risks {
        markdown.push_str(&format!("- {line}\n"));
    }
    markdown.push('\n');

    markdown.push_str("## 下阶段建议\n\n");
    if average_session_duration > 0 && average_session_duration < 20 * 60 {
        markdown.push_str("- 给复杂任务预留 45 到 90 分钟的完整时间块，减少碎片切换。\n");
    } else {
        markdown.push_str("- 延续当前节奏，把高价值任务继续放到连续时间段里推进。\n");
    }
    if let Some(item) = top_apps.first() {
        markdown.push_str(&format!(
            "- 继续围绕 {} 这个主阵地沉淀产出，避免工具切换带来的分心。\n",
            item.name
        ));
    }

    WeeklyReviewResult {
        title,
        markdown,
        total_duration,
        active_days,
        session_count,
        deep_work_sessions,
        top_intents,
        top_apps,
        highlights,
        risks,
    }
}

pub fn extract_todos(activities: &[Activity]) -> TodoExtractionResult {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for activity in activities {
        let sources = build_todo_sources(activity);
        for (candidate, confidence, reason) in sources {
            let normalized = normalize_candidate(&candidate);
            if normalized.len() < 4 || !seen.insert(normalized) {
                continue;
            }

            items.push(TodoItem {
                title: candidate,
                date: date_from_timestamp(activity.timestamp),
                source_title: truncate_text(&activity.window_title, TODO_TEXT_LIMIT),
                source_app: activity.app_name.clone(),
                confidence,
                reason,
            });
        }
    }

    items.sort_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then_with(|| b.date.cmp(&a.date))
            .then_with(|| a.title.cmp(&b.title))
    });
    items.truncate(20);

    let summary = if items.is_empty() {
        "当前时间范围内没有提取到明确的待办信号。".to_string()
    } else {
        format!("共提取到 {} 条候选待办，已按置信度排序。", items.len())
    };

    TodoExtractionResult { items, summary }
}

fn classify_session(
    dominant_category: &str,
    dominant_app: &str,
    title: &str,
    browser_domains: &[String],
    top_keywords: &[String],
    combined_lines: &[String],
) -> IntentMatch {
    let lower_app = dominant_app.to_lowercase();
    let lower_title = title.to_lowercase();
    let domain_text = browser_domains.join(" ").to_lowercase();
    let keyword_text = top_keywords.join(" ").to_lowercase();
    let corpus = combined_lines.join(" ").to_lowercase();

    let mut matches = vec![
        score_intent(
            "编码开发",
            &corpus,
            &[
                "cursor",
                "vscode",
                "visual studio code",
                "code",
                "pycharm",
                "intellij",
                "xcode",
                "webstorm",
                "terminal",
                "iterm",
                "warp",
                "git",
                "commit",
                "branch",
                "merge",
                "cargo",
                "npm",
                "pnpm",
                "debug",
                "feature",
                "代码",
                "开发",
            ],
            12,
        ),
        score_intent(
            "代码评审",
            &corpus,
            &[
                "pull request",
                "merge request",
                "code review",
                "review",
                "diff",
                "approval",
                "comment",
                "pr ",
                " mr ",
                "评审",
                "审查",
            ],
            14,
        ),
        score_intent(
            "需求文档",
            &corpus,
            &[
                "prd",
                "spec",
                "doc",
                "docs",
                "document",
                "notion",
                "语雀",
                "飞书文档",
                "confluence",
                "需求",
                "文档",
                "方案",
                "设计稿",
            ],
            10,
        ),
        score_intent(
            "会议沟通",
            &corpus,
            &[
                "zoom",
                "meeting",
                "meet",
                "teams",
                "slack",
                "discord",
                "飞书",
                "企业微信",
                "会议",
                "沟通",
                "同步",
                "站会",
                "腾讯会议",
            ],
            12,
        ),
        score_intent(
            "问题排查",
            &corpus,
            &[
                "bug",
                "issue",
                "error",
                "exception",
                "traceback",
                "stack trace",
                "failed",
                "failure",
                "日志",
                "报错",
                "修复",
                "排查",
                "异常",
            ],
            12,
        ),
        score_intent(
            "测试验证",
            &corpus,
            &[
                "test",
                "pytest",
                "vitest",
                "playwright",
                "cypress",
                "junit",
                "assert",
                "验证",
                "回归",
                "测试",
                "单测",
            ],
            10,
        ),
        score_intent(
            "学习调研",
            &corpus,
            &[
                "google",
                "stackoverflow",
                "docs",
                "documentation",
                "guide",
                "tutorial",
                "research",
                "readme",
                "调研",
                "教程",
                "文档",
                "资料",
            ],
            8,
        ),
        score_intent(
            "AI 协作",
            &corpus,
            &[
                "chatgpt",
                "openai",
                "claude",
                "deepseek",
                "kimi",
                "gemini",
                "copilot",
                "cursor",
                "llm",
                "prompt",
                "大模型",
            ],
            9,
        ),
        score_intent(
            "项目管理",
            &corpus,
            &[
                "jira",
                "linear",
                "asana",
                "trello",
                "roadmap",
                "milestone",
                "todoist",
                "sprint",
                "任务",
                "排期",
                "看板",
                "项目",
            ],
            10,
        ),
    ];

    if dominant_category == "development" {
        add_score(&mut matches, "编码开发", 10, "主类别为 development");
    }
    if lower_app.contains("browser") || lower_app.contains("chrome") || lower_app.contains("safari")
    {
        add_score(&mut matches, "学习调研", 4, "会话主要发生在浏览器");
    }
    if lower_title.contains("pr") || lower_title.contains("review") {
        add_score(&mut matches, "代码评审", 8, format!("标题包含 {}", title));
    }
    if domain_text.contains("github.com") {
        add_score(&mut matches, "编码开发", 6, "包含 github.com");
        add_score(&mut matches, "代码评审", 4, "包含 github.com");
    }
    if domain_text.contains("openai.com")
        || domain_text.contains("claude.ai")
        || domain_text.contains("chat.deepseek.com")
    {
        add_score(&mut matches, "AI 协作", 10, "包含 AI 工具站点");
    }
    if keyword_text.contains("bug") || keyword_text.contains("报错") {
        add_score(&mut matches, "问题排查", 8, "关键词显示异常处理");
    }

    matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));
    let best = matches
        .into_iter()
        .find(|item| item.score > 0)
        .unwrap_or_else(|| IntentMatch {
            label: "通用工作".to_string(),
            score: 0,
            evidence: vec!["未命中明确意图信号，按通用工作处理".to_string()],
        });

    IntentMatch {
        label: best.label,
        score: if best.score <= 0 { 40 } else { 40 + best.score },
        evidence: best.evidence.into_iter().take(4).collect(),
    }
}

fn score_intent(label: &str, corpus: &str, patterns: &[&str], score_per_hit: i32) -> IntentMatch {
    let mut evidence = Vec::new();
    let mut score = 0;

    for pattern in patterns {
        if corpus.contains(pattern) {
            score += score_per_hit;
            evidence.push(format!("命中关键词 {}", pattern));
        }
    }

    IntentMatch {
        label: label.to_string(),
        score,
        evidence,
    }
}

fn add_score<S: Into<String>>(matches: &mut [IntentMatch], label: &str, delta: i32, evidence: S) {
    if let Some(item) = matches.iter_mut().find(|item| item.label == label) {
        item.score += delta;
        item.evidence.push(evidence.into());
    }
}

fn build_todo_sources(activity: &Activity) -> Vec<(String, i32, String)> {
    let mut candidates = Vec::new();
    let mut texts = vec![activity.window_title.clone()];
    if let Some(ocr_text) = &activity.ocr_text {
        texts.push(ocr_text.clone());
    }
    if let Some(url) = &activity.browser_url {
        texts.push(url.clone());
    }

    let checkbox_regex = checkbox_regex();
    let explicit_regex = explicit_todo_regex();
    let action_regex = action_todo_regex();

    for text in texts {
        for capture in checkbox_regex.captures_iter(&text) {
            if let Some(matched) = capture.get(1) {
                candidates.push((
                    truncate_text(matched.as_str(), TODO_TEXT_LIMIT),
                    90,
                    "命中未完成清单项".to_string(),
                ));
            }
        }

        for capture in explicit_regex.captures_iter(&text) {
            // capture.get(2) 取实际待办文本，跳过 "TODO:" / "待办：" 等前缀
            if let Some(matched) = capture.get(2).or_else(|| capture.get(0)) {
                candidates.push((
                    truncate_text(&clean_todo_text(matched.as_str()), TODO_TEXT_LIMIT),
                    82,
                    "命中显式待办关键词".to_string(),
                ));
            }
        }

        for capture in action_regex.captures_iter(&text) {
            if let Some(matched) = capture.get(0) {
                candidates.push((
                    truncate_text(&clean_todo_text(matched.as_str()), TODO_TEXT_LIMIT),
                    68,
                    "命中动作型任务短语".to_string(),
                ));
            }
        }
    }

    let title_lower = activity.window_title.to_lowercase();
    if title_lower.contains("issue")
        || title_lower.contains("bug")
        || title_lower.contains("pr #")
        || title_lower.contains("任务")
    {
        candidates.push((
            truncate_text(&activity.window_title, TODO_TEXT_LIMIT),
            60,
            "窗口标题像是待跟进事项".to_string(),
        ));
    }

    candidates
}

fn checkbox_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"(?m)\[[ xX]?\]\s*([^\n]{2,80})").expect("checkbox regex"))
}

fn explicit_todo_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)(todo|待办|follow[- ]?up|next step|后续动作|需要跟进)\s*[:：-]?\s*([^\n。；]{2,80})",
        )
        .expect("explicit todo regex")
    })
}

fn action_todo_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)(修复|排查|优化|补充|整理|确认|同步|review|fix|investigate|refactor|verify)\s*[:：-]?\s*([^\n。；]{0,60})",
        )
        .expect("action todo regex")
    })
}

fn extract_keywords(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && !is_cjk(c))
        .map(str::trim)
        .filter(|token| token.chars().count() >= 2)
        .map(|token| token.to_lowercase())
        .filter(|token| !is_noise_token(token))
        .collect()
}

fn is_noise_token(token: &str) -> bool {
    matches!(
        token,
        "https"
            | "http"
            | "www"
            | "com"
            | "the"
            | "and"
            | "with"
            | "for"
            | "from"
            | "into"
            | "main"
            | "work"
            | "review"
            | "页面"
            | "窗口"
            | "文件"
            | "今天"
            | "进行"
            | "正在"
    )
}

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn top_name_by_duration(values: &HashMap<String, i64>) -> Option<String> {
    values
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(name, _)| name.clone())
}

fn top_name_by_count(values: &HashMap<String, i32>) -> Option<String> {
    values
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(name, _)| name.clone())
}

fn top_named_durations(values: &HashMap<String, i64>, limit: usize) -> Vec<NamedDuration> {
    let mut items = values
        .iter()
        .map(|(name, duration)| NamedDuration {
            name: name.clone(),
            duration: *duration,
        })
        .collect::<Vec<_>>();

    items.sort_by_key(|item| (Reverse(item.duration), item.name.clone()));
    items.truncate(limit);
    items
}

fn top_keywords(keyword_counts: &HashMap<String, i32>, limit: usize) -> Vec<String> {
    let mut items = keyword_counts.iter().collect::<Vec<_>>();
    items.sort_by(|a, b| {
        b.1.cmp(a.1)
            .then_with(|| a.0.len().cmp(&b.0.len()))
            .then_with(|| a.0.cmp(b.0))
    });
    items
        .into_iter()
        .take(limit)
        .map(|(keyword, _)| keyword.clone())
        .collect()
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut iter = value.trim().chars();
    let excerpt = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{excerpt}…")
    } else {
        excerpt
    }
}

fn clean_todo_text(value: &str) -> String {
    value
        .replace(['\n', '\r'], " ")
        .replace("[]", "")
        .replace("[ ]", "")
        .replace("[x]", "")
        .trim_matches(|c: char| c == ':' || c == '：' || c == '-' || c == ' ')
        .trim()
        .to_string()
}

fn normalize_candidate(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric() || is_cjk(*ch))
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn extract_domain(url: &str) -> String {
    url.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn date_from_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .earliest()
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{analyze_intents, build_work_sessions, extract_todos};
    use crate::database::Activity;

    fn activity(
        timestamp: i64,
        app_name: &str,
        title: &str,
        duration: i64,
        browser_url: Option<&str>,
        ocr_text: Option<&str>,
    ) -> Activity {
        Activity {
            id: None,
            timestamp,
            app_name: app_name.to_string(),
            window_title: title.to_string(),
            screenshot_path: String::new(),
            ocr_text: ocr_text.map(|value| value.to_string()),
            category: "development".to_string(),
            duration,
            browser_url: browser_url.map(|value| value.to_string()),
            executable_path: None,
            semantic_category: None,
            semantic_confidence: None,
        }
    }

    #[test]
    fn 应按空闲间隔聚合连续_session() {
        let activities = vec![
            activity(1_700_000_000, "Code", "实现 session 聚合", 300, None, None),
            activity(1_700_000_360, "iTerm", "cargo test", 240, None, None),
            activity(
                1_700_002_000,
                "Chrome",
                "GitHub Pull Request",
                180,
                Some("https://github.com/org/repo/pull/1"),
                None,
            ),
        ];

        let sessions = build_work_sessions(&activities);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[1].activity_count, 2);
        assert_eq!(sessions[1].dominant_app, "Code");
        assert_eq!(sessions[0].browser_domains, vec!["github.com"]);
    }

    #[test]
    fn 应基于会话识别主要意图() {
        let activities = vec![
            activity(
                1_700_000_000,
                "Code",
                "fix login bug",
                300,
                None,
                Some("traceback investigate error"),
            ),
            activity(
                1_700_000_200,
                "Chrome",
                "GitHub Pull Request review",
                240,
                Some("https://github.com/org/repo/pull/8"),
                None,
            ),
        ];

        let result = analyze_intents(&activities);
        assert_eq!(result.sessions.len(), 1);
        assert!(
            result.sessions[0].intent_label == "问题排查"
                || result.sessions[0].intent_label == "代码评审"
                || result.sessions[0].intent_label == "编码开发"
        );
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn 应提取显式待办项() {
        let activities = vec![
            activity(
                1_700_000_000,
                "Notion",
                "TODO: 补充周报结论",
                120,
                None,
                Some("- [ ] 修复 session 聚合边界"),
            ),
            activity(
                1_700_000_200,
                "Chrome",
                "Issue #42 fix login redirect",
                120,
                Some("https://github.com/org/repo/issues/42"),
                None,
            ),
        ];

        let todos = extract_todos(&activities);
        assert!(!todos.items.is_empty());
        assert!(todos.items.iter().any(|item| item.title.contains("修复")));
    }
}
