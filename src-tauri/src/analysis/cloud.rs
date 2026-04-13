use crate::analysis::{
    append_custom_prompt_for_locale, generate_stats_summary_for_locale, Analyzer, AppLocale,
    GeneratedReport,
};
use crate::database::{Activity, DailyStats};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use reqwest::Client;
use serde_json::json;
use std::path::Path;
use std::time::Duration;

fn screenshot_prompt(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "请简要描述这张截图中的工作内容，用简体中文回答，限制在 50 字以内。",
        AppLocale::ZhTw => "請簡要描述這張截圖中的工作內容，請用繁體中文回答，限制在 50 字內。",
        AppLocale::En => "Briefly describe the work shown in this screenshot in under 50 words.",
    }
}

fn report_system_prompt(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "你是一位有温度的工作日报助手，擅长将一天的工作记录整理成自然、具体、可信的回顾。",
        AppLocale::ZhTw => "你是一位有溫度的工作日報助手，擅長將一天的工作記錄整理成自然、具體、可信的回顧。",
        AppLocale::En => "You are a thoughtful daily work-report assistant. Turn the user's work records into a natural, concrete, and trustworthy recap.",
    }
}

/// 云端视觉分析器
pub struct CloudAnalyzer {
    api_key: String,
    endpoint: String,
    model: String,
    custom_prompt: String,
    locale: AppLocale,
    client: Client,
}

impl CloudAnalyzer {
    pub fn new(
        endpoint: &str,
        api_key: &str,
        model: &str,
        custom_prompt: &str,
        locale: AppLocale,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            api_key: api_key.to_string(),
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            custom_prompt: custom_prompt.to_string(),
            locale,
            client,
        }
    }

    async fn analyze_screenshot(&self, screenshot_path: &Path) -> Result<String> {
        let image_data = tokio::fs::read(screenshot_path).await?;
        let image_base64 = BASE64_STANDARD.encode(&image_data);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "text",
                                "text": screenshot_prompt(self.locale)
                            },
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:image/png;base64,{}", image_base64),
                                    "detail": "low"
                                }
                            }
                        ]
                    }
                ],
                "max_tokens": 100,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Analysis(format!(
                "OpenAI Vision API 错误: {error_text}"
            )));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("无法分析")
            .to_string())
    }

    async fn generate_final_report(
        &self,
        date: &str,
        stats: &DailyStats,
        insights: &[String],
    ) -> Result<String> {
        let stats_summary = generate_stats_summary_for_locale(stats, self.locale);
        let insights_text = insights
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{}. {}", index + 1, item))
            .collect::<Vec<_>>()
            .join("\n");

        let base_prompt = match self.locale {
            AppLocale::ZhCn => format!(
                "以下是一位用户今天的工作数据：\n\n{stats_summary}\n\n### 从截图识别到的工作内容\n{insights_text}\n\n请据此生成一份有价值的工作日报，使用简体中文 Markdown 输出。",
            ),
            AppLocale::ZhTw => format!(
                "以下是一位使用者今天的工作資料：\n\n{stats_summary}\n\n### 從截圖辨識到的工作內容\n{insights_text}\n\n請據此生成一份有價值的工作日報，使用繁體中文 Markdown 輸出。",
            ),
            AppLocale::En => format!(
                "Below is a user's work data for today:\n\n{stats_summary}\n\n### Work content identified from screenshots\n{insights_text}\n\nWrite a valuable daily work report in English Markdown based on this information.",
            ),
        };
        let prompt = append_custom_prompt_for_locale(base_prompt, &self.custom_prompt, self.locale);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "system",
                        "content": report_system_prompt(self.locale)
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "max_tokens": 2000,
                "temperature": 0.8,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Analysis(format!("OpenAI API 错误: {error_text}")));
        }

        let result: serde_json::Value = response.json().await?;
        let report = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(match self.locale {
            AppLocale::ZhCn => format!("# 工作日报 - {date}\n\n{report}"),
            AppLocale::ZhTw => format!("# 工作日報 - {date}\n\n{report}"),
            AppLocale::En => format!("# Daily Report - {date}\n\n{report}"),
        })
    }
}

#[async_trait]
impl Analyzer for CloudAnalyzer {
    async fn generate_report(
        &self,
        date: &str,
        stats: &DailyStats,
        activities: &[Activity],
        screenshots_dir: &Path,
        _locale: AppLocale,
    ) -> Result<GeneratedReport> {
        if self.api_key.is_empty() {
            return Err(AppError::Analysis("OpenAI API Key 未配置".to_string()));
        }

        let sample_size = std::cmp::min(activities.len(), 5);
        let step = if activities.len() > sample_size {
            activities.len() / sample_size
        } else {
            1
        };

        let mut insights = Vec::new();
        for (index, activity) in activities.iter().enumerate() {
            if insights.len() >= sample_size {
                break;
            }
            if index % step != 0 {
                continue;
            }

            let screenshot_path = screenshots_dir.join(&activity.screenshot_path);
            if screenshot_path.exists() {
                if let Ok(insight) = self.analyze_screenshot(&screenshot_path).await {
                    insights.push(format!("[{}] {}", activity.app_name, insight));
                }
            }
        }

        Ok(GeneratedReport {
            content: self.generate_final_report(date, stats, &insights).await?,
            used_ai: true,
            fallback_reason: None,
        })
    }
}
