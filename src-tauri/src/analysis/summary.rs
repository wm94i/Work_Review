use crate::analysis::{
    append_custom_prompt_for_locale, format_duration_for_locale,
    generate_hourly_activity_summary_for_locale, translate_category_name,
    translate_semantic_category_name, Analyzer, AppLocale, GeneratedReport,
};
use crate::config::AiProvider;
use crate::database::{Activity, DailyStats};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use reqwest::{Client, Url};
use serde_json::json;
use std::path::Path;
use std::time::Duration;

fn summary_request_timeout(provider: AiProvider, endpoint: &str) -> Duration {
    if provider == AiProvider::Ollama || is_local_summary_endpoint(endpoint) {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(90)
    }
}

fn is_local_summary_endpoint(endpoint: &str) -> bool {
    Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| {
            matches!(
                host.as_str(),
                "localhost" | "127.0.0.1" | "::1" | "[::1]" | "0.0.0.0"
            )
        })
}

fn format_domain_label(domain: &crate::database::DomainUsage, locale: AppLocale) -> String {
    match domain.semantic_category.as_deref().map(str::trim) {
        Some(semantic_category) if !semantic_category.is_empty() => {
            let semantic_category = translate_semantic_category_name(semantic_category, locale);
            match locale {
                AppLocale::En => format!("{} ({})", domain.domain, semantic_category),
                _ => format!("{}（{}）", domain.domain, semantic_category),
            }
        }
        _ => domain.domain.clone(),
    }
}

fn empty_value(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "无",
        AppLocale::ZhTw => "無",
        AppLocale::En => "None",
    }
}

fn join_list(locale: AppLocale, items: Vec<String>) -> String {
    items.join(if locale == AppLocale::En { ", " } else { "、" })
}

fn ai_system_prompt(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => {
            "你是一个专业的工作效率分析助手，帮助用户分析和总结每日工作。请使用简体中文回答。"
        }
        AppLocale::ZhTw => {
            "你是一位專業的工作效率分析助手，負責協助使用者分析與總結每日工作。請使用繁體中文回答。"
        }
        AppLocale::En => {
            "You are a professional work-efficiency analysis assistant. Summarize and analyze the user's workday in English."
        }
    }
}

/// 摘要上传分析器
/// 只上传统计摘要，不上传原始截图
pub struct SummaryAnalyzer {
    provider: AiProvider,
    endpoint: String,
    model: String,
    api_key: Option<String>,
    custom_prompt: String,
    locale: AppLocale,
    client: Client,
}

impl SummaryAnalyzer {
    pub fn new(
        provider: AiProvider,
        endpoint: &str,
        model: &str,
        api_key: Option<&str>,
        custom_prompt: &str,
        locale: AppLocale,
    ) -> Self {
        let client = Client::builder()
            .timeout(summary_request_timeout(provider, endpoint))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            provider,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            api_key: api_key.map(|value| value.to_string()),
            custom_prompt: custom_prompt.to_string(),
            locale,
            client,
        }
    }

    async fn generate_with_ollama(&self, prompt: &str) -> Result<String> {
        let response = self
            .client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&json!({
                "model": self.model,
                "prompt": format!("{}\n\n{}", ai_system_prompt(self.locale), prompt),
                "stream": false,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Analysis(format!(
                "Ollama API 错误: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["response"].as_str().unwrap_or("").trim().to_string())
    }

    async fn generate_with_openai_compatible(&self, prompt: &str) -> Result<String> {
        let mut request = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "system",
                        "content": ai_system_prompt(self.locale)
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "max_tokens": 5000,
                "temperature": 0.2,
            }));

        if let Some(api_key) = &self.api_key {
            if !api_key.is_empty() {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }
        }

        let response = request.send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Analysis(format!("API 错误: {error_text}")));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string())
    }

    async fn generate_with_claude(&self, prompt: &str) -> Result<String> {
        let api_key = self.api_key.as_deref().unwrap_or("");
        if api_key.is_empty() {
            return Err(AppError::Analysis("Claude API Key 未配置".to_string()));
        }

        let response = self
            .client
            .post(format!("{}/messages", self.endpoint))
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": self.model,
                "max_tokens": 5000,
                "messages": [
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "system": ai_system_prompt(self.locale)
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Analysis(format!("Claude API 错误: {error_text}")));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string())
    }

    async fn generate_with_gemini(&self, prompt: &str) -> Result<String> {
        let api_key = self.api_key.as_deref().unwrap_or("");
        if api_key.is_empty() {
            return Err(AppError::Analysis("Gemini API Key 未配置".to_string()));
        }

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.endpoint, self.model, api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&json!({
                "contents": [{
                    "parts": [{
                        "text": format!("{}\n\n{}", ai_system_prompt(self.locale), prompt)
                    }]
                }],
                "generationConfig": {
                    "temperature": 0.2,
                    "maxOutputTokens": 5000
                }
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Analysis(format!("Gemini API 错误: {error_text}")));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string())
    }

    async fn generate_ai_content(&self, prompt: &str) -> Result<String> {
        match self.provider {
            AiProvider::Ollama => self.generate_with_ollama(prompt).await,
            AiProvider::Claude => self.generate_with_claude(prompt).await,
            AiProvider::Gemini => self.generate_with_gemini(prompt).await,
            _ => self.generate_with_openai_compatible(prompt).await,
        }
    }

    fn extract_keywords(&self, activities: &[Activity]) -> Vec<String> {
        let mut keywords = Vec::new();

        for activity in activities {
            let title_words = activity
                .window_title
                .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .filter(|word| word.len() > 3)
                .take(3)
                .collect::<Vec<_>>();

            for word in title_words {
                let item = word.to_string();
                if !keywords.contains(&item) {
                    keywords.push(item);
                }
            }

            if let Some(ocr_text) = &activity.ocr_text {
                let ocr_words = ocr_text
                    .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                    .filter(|word| {
                        word.len() > 3
                            && word
                                .chars()
                                .all(|char| char.is_alphabetic() || char >= '\u{4e00}')
                    })
                    .take(5)
                    .collect::<Vec<_>>();

                for word in ocr_words {
                    let item = word.to_string();
                    if !keywords.contains(&item) && keywords.len() < 30 {
                        keywords.push(item);
                    }
                }
            }
        }

        keywords.truncate(30);
        keywords
    }

    fn build_ai_prompt(&self, date: &str, stats: &DailyStats, activities: &[Activity]) -> String {
        let apps_list = join_list(
            self.locale,
            stats
                .app_usage
                .iter()
                .take(8)
                .map(|app| {
                    format!(
                        "{} ({})",
                        app.app_name,
                        format_duration_for_locale(app.duration, self.locale)
                    )
                })
                .collect(),
        );

        let urls_list = join_list(
            self.locale,
            stats
                .domain_usage
                .iter()
                .take(5)
                .map(|domain| format_domain_label(domain, self.locale))
                .collect(),
        );

        let keywords = self.extract_keywords(activities);
        let top_keywords = join_list(self.locale, keywords.into_iter().take(8).collect());
        let hourly_summary = generate_hourly_activity_summary_for_locale(stats, self.locale)
            .unwrap_or_else(|| match self.locale {
                AppLocale::ZhCn => "暂无按小时活跃度数据".to_string(),
                AppLocale::ZhTw => "暫無按小時活躍度資料".to_string(),
                AppLocale::En => "No hourly activity data available".to_string(),
            });

        let base_prompt = match self.locale {
            AppLocale::ZhCn => format!(
                r#"请基于以下数据，生成一份面向用户的工作日报 AI 分析补充。重点是提炼信息和给出洞察，不要逐条复述原始数据。

【日期】
{date}

【今日原始数据】
工作时长：{}
主要应用：{}
访问网站：{}
按小时活跃度：{}
屏幕内容关键词：{}

【核心要求】
1. 结合应用、网站和关键词，推断今天的工作重心。
2. 结合时长分布判断专注状态和节奏。
3. 给出 1 到 2 条具体建议。
4. 如果某项数据缺失，请明确说明未获取到，不要编造。

【输出格式】
请严格按以下四个标题输出，并使用简体中文：

**工作内容概述**

**效率评估**

**改进建议**

**今日小结**"#,
                format_duration_for_locale(stats.total_duration, self.locale),
                if apps_list.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    apps_list
                },
                if urls_list.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    urls_list
                },
                hourly_summary,
                if top_keywords.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    top_keywords
                }
            ),
            AppLocale::ZhTw => format!(
                r#"請根據以下資料，生成一份面向使用者的工作日報 AI 分析補充。重點是提煉資訊與給出洞察，不要逐條重述原始資料。

【日期】
{date}

【今日原始資料】
工作時長：{}
主要應用：{}
造訪網站：{}
按小時活躍度：{}
畫面內容關鍵詞：{}

【核心要求】
1. 結合應用、網站與關鍵詞，推斷今天的工作重心。
2. 結合時長分布判斷專注狀態與節奏。
3. 給出 1 到 2 條具體建議。
4. 如果某項資料缺失，請明確說明未取得，不要編造。

【輸出格式】
請嚴格按以下四個標題輸出，並使用繁體中文：

**工作內容概述**

**效率評估**

**改進建議**

**今日小結**"#,
                format_duration_for_locale(stats.total_duration, self.locale),
                if apps_list.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    apps_list
                },
                if urls_list.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    urls_list
                },
                hourly_summary,
                if top_keywords.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    top_keywords
                }
            ),
            AppLocale::En => format!(
                r#"Use the data below to write the AI analysis section of a daily work report. Focus on insight and synthesis rather than repeating raw numbers line by line.

[Date]
{date}

[Raw data]
Work duration: {}
Main apps: {}
Visited websites: {}
Hourly activity: {}
Screen-content keywords: {}

[Requirements]
1. Infer the user's main work focus from apps, websites, and keywords.
2. Assess focus and rhythm from the time distribution.
3. Give 1 to 2 concrete suggestions.
4. If a data point is missing, say so clearly instead of making it up.

[Output format]
Write in English and use exactly these four section headings:

**Work Summary**

**Efficiency Assessment**

**Improvement Suggestions**

**Daily Wrap-up**"#,
                format_duration_for_locale(stats.total_duration, self.locale),
                if apps_list.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    apps_list
                },
                if urls_list.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    urls_list
                },
                hourly_summary,
                if top_keywords.is_empty() {
                    empty_value(self.locale).to_string()
                } else {
                    top_keywords
                }
            ),
        };

        append_custom_prompt_for_locale(base_prompt, &self.custom_prompt, self.locale)
    }

    fn generate_fallback_ai_content(&self, apps_list: &str) -> String {
        match self.locale {
            AppLocale::ZhCn => format!(
                "## 工作内容概述\n\n今天主要围绕 {} 等工具推进工作，整体工作主线比较清晰。\n\n## 效率评估\n\n当前记录显示今天的工作节奏相对稳定，但仍可以继续优化连续专注时间。\n\n## 改进建议\n\n建议为最重要的任务预留更完整的连续时间段，减少中途切换。\n\n## 今日小结\n\n今天保持了稳定推进，已经积累了不错的工作产出。\n\n---\n*注：由基础模板生成。配置 AI 模型后可获得更深入的智能分析。*",
                if apps_list.is_empty() { "多个应用".to_string() } else { apps_list.to_string() }
            ),
            AppLocale::ZhTw => format!(
                "## 工作內容概述\n\n今天主要圍繞 {} 等工具推進工作，整體工作主線相對清晰。\n\n## 效率評估\n\n目前記錄顯示今天的工作節奏較穩定，但仍可持續優化連續專注時間。\n\n## 改進建議\n\n建議為最重要的任務預留更完整的連續時間段，減少中途切換。\n\n## 今日小結\n\n今天維持了穩定推進，已經累積了不錯的工作產出。\n\n---\n*註：目前由基礎模板生成。配置 AI 模型後可獲得更深入的智慧分析。*",
                if apps_list.is_empty() { "多個應用".to_string() } else { apps_list.to_string() }
            ),
            AppLocale::En => format!(
                "## Work Summary\n\nToday's work mainly revolved around tools such as {}, and the overall direction stayed fairly clear.\n\n## Efficiency Assessment\n\nThe recorded activity suggests a steady working rhythm today, though there is still room to improve uninterrupted focus time.\n\n## Improvement Suggestions\n\nReserve a longer uninterrupted block for the most important task to reduce context switching.\n\n## Daily Wrap-up\n\nThe day moved forward at a stable pace and produced solid progress.\n\n---\n*Note: This section was generated from the base template because AI analysis was unavailable.*",
                if apps_list.is_empty() { "several apps".to_string() } else { apps_list.to_string() }
            ),
        }
    }
}

#[async_trait]
impl Analyzer for SummaryAnalyzer {
    async fn generate_report(
        &self,
        date: &str,
        stats: &DailyStats,
        activities: &[Activity],
        _screenshots_dir: &Path,
        locale: AppLocale,
    ) -> Result<GeneratedReport> {
        log::info!("生成混合模式日报：固定模板 + AI 扩展");

        let mut report = String::new();

        match locale {
            AppLocale::ZhCn => {
                report.push_str(&format!("# 工作日报\n\n**日期：{date}**\n\n"));
                report.push_str("## 一、今日概览\n\n");
                report.push_str("| 指标 | 数值 |\n|:--|--:|\n");
                report.push_str(&format!(
                    "| 总工作时长 | {} |\n| 截图数量 | {} 张 |\n| 使用应用数 | {} 个 |\n| 访问网站数 | {} 个 |\n\n",
                    format_duration_for_locale(stats.total_duration, locale),
                    stats.screenshot_count,
                    stats.app_usage.len(),
                    stats.domain_usage.len()
                ));
            }
            AppLocale::ZhTw => {
                report.push_str(&format!("# 工作日報\n\n**日期：{date}**\n\n"));
                report.push_str("## 一、今日概覽\n\n");
                report.push_str("| 指標 | 數值 |\n|:--|--:|\n");
                report.push_str(&format!(
                    "| 總工作時長 | {} |\n| 截圖數量 | {} 張 |\n| 使用應用數 | {} 個 |\n| 造訪網站數 | {} 個 |\n\n",
                    format_duration_for_locale(stats.total_duration, locale),
                    stats.screenshot_count,
                    stats.app_usage.len(),
                    stats.domain_usage.len()
                ));
            }
            AppLocale::En => {
                report.push_str(&format!("# Daily Report\n\n**Date:** {date}\n\n"));
                report.push_str("## 1. Overview\n\n");
                report.push_str("| Metric | Value |\n|:--|--:|\n");
                report.push_str(&format!(
                    "| Total work duration | {} |\n| Screenshot count | {} |\n| Apps used | {} |\n| Websites visited | {} |\n\n",
                    format_duration_for_locale(stats.total_duration, locale),
                    stats.screenshot_count,
                    stats.app_usage.len(),
                    stats.domain_usage.len()
                ));
            }
        }

        if !stats.category_usage.is_empty() {
            match locale {
                AppLocale::ZhCn => {
                    report.push_str("## 二、时间分配\n\n| 类别 | 时长 | 占比 |\n|:--|--:|--:|\n");
                }
                AppLocale::ZhTw => {
                    report.push_str("## 二、時間分配\n\n| 類別 | 時長 | 佔比 |\n|:--|--:|--:|\n");
                }
                AppLocale::En => {
                    report.push_str(
                        "## 2. Time Allocation\n\n| Category | Duration | Share |\n|:--|--:|--:|\n",
                    );
                }
            }

            for cat in &stats.category_usage {
                let percentage = if stats.total_duration > 0 {
                    (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
                } else {
                    0
                };
                report.push_str(&format!(
                    "| {} | {} | {}% |\n",
                    translate_category_name(&cat.category, locale),
                    format_duration_for_locale(cat.duration, locale),
                    percentage
                ));
            }
            report.push('\n');
        }

        if !stats.app_usage.is_empty() {
            report.push_str(match locale {
                AppLocale::ZhCn => {
                    "## 三、应用使用明细\n\n| 序号 | 应用名称 | 使用时长 |\n|--:|:--|--:|\n"
                }
                AppLocale::ZhTw => {
                    "## 三、應用使用明細\n\n| 序號 | 應用名稱 | 使用時長 |\n|--:|:--|--:|\n"
                }
                AppLocale::En => "## 3. App Details\n\n| # | App | Duration |\n|--:|:--|--:|\n",
            });
            for (index, app) in stats.app_usage.iter().enumerate() {
                report.push_str(&format!(
                    "| {} | {} | {} |\n",
                    index + 1,
                    app.app_name,
                    format_duration_for_locale(app.duration, locale)
                ));
            }
            report.push('\n');
        }

        if let Some(hourly_summary) = generate_hourly_activity_summary_for_locale(stats, locale) {
            report.push_str(match locale {
                AppLocale::ZhCn => "## 四、按小时活跃度\n\n",
                AppLocale::ZhTw => "## 四、按小時活躍度\n\n",
                AppLocale::En => "## 4. Hourly Activity\n\n",
            });
            report.push_str(&hourly_summary);
            report.push('\n');
        }

        if !stats.domain_usage.is_empty() {
            report.push_str(match locale {
                AppLocale::ZhCn => {
                    "## 五、网站访问明细\n\n| 序号 | 网站域名 | 访问时长 |\n|--:|:--|--:|\n"
                }
                AppLocale::ZhTw => {
                    "## 五、網站造訪明細\n\n| 序號 | 網站網域 | 造訪時長 |\n|--:|:--|--:|\n"
                }
                AppLocale::En => {
                    "## 5. Website Details\n\n| # | Domain | Duration |\n|--:|:--|--:|\n"
                }
            });
            for (index, domain) in stats.domain_usage.iter().enumerate() {
                report.push_str(&format!(
                    "| {} | {} | {} |\n",
                    index + 1,
                    format_domain_label(domain, locale),
                    format_duration_for_locale(domain.duration, locale)
                ));
            }
            report.push('\n');
        }

        report.push_str(match locale {
            AppLocale::ZhCn => "## 六、AI 分析\n\n",
            AppLocale::ZhTw => "## 六、AI 分析\n\n",
            AppLocale::En => "## 6. AI Analysis\n\n",
        });

        let apps_list = join_list(
            locale,
            stats
                .app_usage
                .iter()
                .take(8)
                .map(|app| {
                    format!(
                        "{} ({})",
                        app.app_name,
                        format_duration_for_locale(app.duration, locale)
                    )
                })
                .collect(),
        );

        let ai_content = match self
            .generate_ai_content(&self.build_ai_prompt(date, stats, activities))
            .await
        {
            Ok(content) if !content.is_empty() => (content, true),
            Ok(_) | Err(_) => (self.generate_fallback_ai_content(&apps_list), false),
        };

        report.push_str(&ai_content.0);

        Ok(GeneratedReport {
            content: report,
            used_ai: ai_content.1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::summary_request_timeout;
    use crate::config::AiProvider;
    use std::time::Duration;

    #[test]
    fn 本地_openai兼容端点应使用更长的日报生成超时() {
        assert_eq!(
            summary_request_timeout(AiProvider::OpenAI, "http://127.0.0.1:1234/v1"),
            Duration::from_secs(300)
        );
        assert_eq!(
            summary_request_timeout(AiProvider::OpenAI, "http://localhost:1234/v1"),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn 本地_ollama端点应使用更长的日报生成超时() {
        assert_eq!(
            summary_request_timeout(AiProvider::Ollama, "http://localhost:11434"),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn 远端摘要日报接口应保持原有超时() {
        assert_eq!(
            summary_request_timeout(AiProvider::OpenAI, "https://api.openai.com/v1"),
            Duration::from_secs(90)
        );
        assert_eq!(
            summary_request_timeout(AiProvider::Gemini, "https://generativelanguage.googleapis.com/v1"),
            Duration::from_secs(90)
        );
    }
}
