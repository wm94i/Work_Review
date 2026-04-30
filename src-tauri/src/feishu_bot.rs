use crate::config::{AppConfig, DEFAULT_LOCALHOST_API_PORT};
use crate::localhost_api::LOCALHOST_API_HOST;
use reqwest::Client;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct DeviceEndpoint {
    name: String,
    url: String,
    token: String,
    is_local: bool,
}

pub struct FeishuResponse {
    pub status: u16,
    pub body: String,
}

impl FeishuResponse {
    pub fn json(status: u16, value: &serde_json::Value) -> Self {
        Self {
            status,
            body: value.to_string(),
        }
    }

    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, &serde_json::json!({"error": message.into()}))
    }
}

// Token cache: (token, expires_at)
static TOKEN_CACHE: Mutex<Option<(String, Instant)>> = Mutex::new(None);

const HELP: &str = "\
📊 Work Review Bot（多设备）

常用命令
/help                      查看帮助
/devices                   查看所有设备
/device [设备名]           查看设备状态（默认本机）
/reports [设备名]          查看最近日报日期
/report [日期] [设备名]    查看指定日报
/generate [日期] [设备名]  生成日报

参数说明
- [设备名] 可选，不填默认本机
- [日期] 可选，不填默认 today
- 日期支持：YYYY-MM-DD / today / yesterday

示例
- /generate today
- /report 2026-04-25
- /reports 本机";
const UNKNOWN_CMD_REPLY: &str = "未知命令。发送 /help 查看帮助，例如：/generate today";
const NON_TEXT_REPLY: &str = "暂不支持非文本消息，发送 /help 查看帮助";
const OUTPUT_DIVIDER: &str = "────────────";

fn effective_host(config: &AppConfig) -> String {
    config
        .localhost_api_host
        .as_deref()
        .map(|h| h.trim())
        .filter(|h| !h.is_empty())
        .unwrap_or(LOCALHOST_API_HOST)
        .to_string()
}

fn read_api_token(data_dir: &Path) -> Option<String> {
    let path = data_dir.join("localhost_api_token.txt");
    std::fs::read_to_string(&path)
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn build_device_list(config: &AppConfig, data_dir: &Path) -> Vec<DeviceEndpoint> {
    let mut devices = Vec::new();
    let host = effective_host(config);
    let port = if config.localhost_api_port == 0 {
        DEFAULT_LOCALHOST_API_PORT
    } else {
        config.localhost_api_port
    };
    if let Some(token) = read_api_token(data_dir) {
        devices.push(DeviceEndpoint {
            name: "本机".to_string(),
            url: format!("http://{}:{}", host, port),
            token,
            is_local: true,
        });
    }
    for d in &config.node_devices {
        devices.push(DeviceEndpoint {
            name: d.name.clone(),
            url: d.url.trim_end_matches('/').to_string(),
            token: d.token.clone(),
            is_local: false,
        });
    }
    devices
}

fn find_device<'a>(devices: &'a [DeviceEndpoint], name: &str) -> Option<&'a DeviceEndpoint> {
    if name.is_empty() || name == "本机" || name == "local" {
        return devices.iter().find(|d| d.is_local);
    }
    devices.iter().find(|d| d.name == name)
}

fn no_available_device_reply() -> String {
    "❌ 无可用设备\n请先启用本地 API 并生成 Token。".to_string()
}

fn connection_failed_reply(device_name: &str) -> String {
    format!("❌ 连接失败\n设备：{device_name}\n请检查地址、Token 与网络连通性。")
}

fn progress_text_for_command(cmd: &str) -> Option<&'static str> {
    match cmd {
        "设备列表" | "devices" => Some("⏳ 正在获取设备列表，请稍候..."),
        "设备" | "device" => Some("⏳ 正在获取设备状态，请稍候..."),
        "日报列表" | "reports" => Some("⏳ 正在获取日报列表，请稍候..."),
        "日报" | "report" => Some("⏳ 正在获取日报详情，请稍候..."),
        "生成日报" | "generate" => Some("⏳ 正在生成日报，预计需要 30-120 秒..."),
        _ => None,
    }
}

async fn get_tenant_token(client: &Client, app_id: &str, app_secret: &str) -> Option<String> {
    // Check cache
    {
        let cache = TOKEN_CACHE.lock().ok()?;
        if let Some((token, expires)) = cache.as_ref() {
            if expires > &Instant::now() {
                return Some(token.clone());
            }
        }
    }
    let resp = client
        .post("https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal")
        .json(&serde_json::json!({"app_id": app_id, "app_secret": app_secret}))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;
    let token = data.get("tenant_access_token")?.as_str()?.to_string();
    let expire = data.get("expire").and_then(|v| v.as_u64()).unwrap_or(7200);
    let cache_ttl = expire.saturating_sub(60);
    if let Ok(mut cache) = TOKEN_CACHE.lock() {
        *cache = Some((
            token.clone(),
            Instant::now() + Duration::from_secs(cache_ttl.max(60)),
        ));
    }
    Some(token)
}

async fn reply_message(client: &Client, token: &str, message_id: &str, text: &str) -> Option<()> {
    let url = format!(
        "https://open.feishu.cn/open-apis/im/v1/messages/{}/reply",
        message_id
    );
    let content = serde_json::json!({"text": text}).to_string();
    client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({"content_type": "text", "content": content}))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    Some(())
}

async fn api_get(client: &Client, url: &str) -> Option<serde_json::Value> {
    client
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?
        .json::<serde_json::Value>()
        .await
        .ok()
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }

    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max).collect();
    if chars.next().is_none() {
        return head;
    }

    let mut trimmed: String = head.chars().take(max.saturating_sub(1)).collect();
    trimmed.push('…');
    trimmed
}

fn normalize_command(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('/')
        .split('@')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn status_payload(status: &str, reason: &str, detail: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "reason": reason,
        "detail": detail.unwrap_or(""),
    })
}

fn parse_table_cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .filter(|cell| !cell.is_empty())
        .collect()
}

fn is_table_separator(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|cell| cell.chars().all(|ch| ch == '-' || ch == ':'))
}

fn normalize_report_for_chat(content: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_table = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut last_non_empty = String::new();

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            if in_table {
                in_table = false;
                table_headers.clear();
            }
            if lines.last().is_some_and(|l| !l.is_empty()) {
                lines.push(String::new());
            }
            continue;
        }

        if line.starts_with('|') && line.ends_with('|') {
            let cells = parse_table_cells(line);
            if cells.is_empty() {
                continue;
            }
            if is_table_separator(&cells) {
                continue;
            }
            if !in_table {
                in_table = true;
                table_headers = cells;
                continue;
            }
            let row =
                if table_headers.first().is_some_and(|h| h.contains("序号")) && cells.len() >= 3 {
                    format!("- {}. {}（{}）", cells[0], cells[1], cells[2])
                } else if cells.len() >= 2 {
                    format!("- {}：{}", cells[0], cells[1..].join(" / "))
                } else {
                    format!("- {}", cells.join(" / "))
                };
            if row != last_non_empty {
                last_non_empty = row.clone();
                lines.push(row);
            }
            continue;
        }

        if in_table {
            in_table = false;
            table_headers.clear();
        }

        let mut converted = line
            .trim_start_matches('#')
            .trim()
            .replace("**", "")
            .replace("*   ", "- ")
            .replace("* ", "- ");
        if converted.starts_with("- - ") {
            converted = converted.replacen("- - ", "- ", 1);
        }
        if converted != last_non_empty {
            last_non_empty = converted.clone();
            lines.push(converted);
        }
    }

    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

async fn handle_cmd(client: &Client, devices: &[DeviceEndpoint], text: &str) -> Option<String> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let cmd = normalize_command(parts.first().copied().unwrap_or(""));
    if cmd.is_empty() {
        return Some(UNKNOWN_CMD_REPLY.to_string());
    }

    // Chinese and English keyword matching
    match cmd.as_str() {
        "帮助" | "help" => Some(HELP.to_string()),
        "设备列表" | "devices" => {
            if devices.is_empty() {
                return Some(no_available_device_reply());
            }
            let mut lines = vec!["🧭 设备列表".to_string(), OUTPUT_DIVIDER.to_string()];
            for (idx, d) in devices.iter().enumerate() {
                let tag = if d.is_local { " (本机)" } else { "" };
                let health = api_get(client, &format!("{}/health", d.url)).await;
                let status = match health {
                    Some(h) if h.get("status").and_then(|v| v.as_str()) == Some("ok") => "✅",
                    Some(_) => "⚠️",
                    None => "❌",
                };
                lines.push(format!("{}. {status} {}{}", idx + 1, d.name, tag));
            }
            Some(lines.join("\n"))
        }
        "设备" | "device" => {
            let device = match find_device(devices, parts.get(1).copied().unwrap_or(""))
                .or_else(|| devices.first())
            {
                Some(d) => d,
                None => return Some(no_available_device_reply()),
            };
            let url = format!("{}/v1/device?token={}", device.url, device.token);
            match api_get(client, &url).await {
                Some(data) => {
                    Some(format!(
                    "🖥 设备状态\n{OUTPUT_DIVIDER}\n设备：{}\nID：{}\n名称：{}\n平台：{}\n录制：{}",
                    device.name,
                    data.get("deviceId").and_then(|v| v.as_str()).unwrap_or("-"),
                    data.get("deviceName").and_then(|v| v.as_str()).unwrap_or("-"),
                    data.get("platform").and_then(|v| v.as_str()).unwrap_or("-"),
                    if data.get("recording").and_then(|v| v.as_bool()).unwrap_or(false) {
                        "是"
                    } else {
                        "否"
                    },
                ))
                }
                None => Some(connection_failed_reply(&device.name)),
            }
        }
        "日报列表" | "reports" => {
            let device = match find_device(devices, parts.get(1).copied().unwrap_or(""))
                .or_else(|| devices.first())
            {
                Some(d) => d,
                None => return Some(no_available_device_reply()),
            };
            let url = format!("{}/v1/reports?token={}&limit=10", device.url, device.token);
            match api_get(client, &url).await {
                Some(json) => {
                    let dates = json.get("dates")?.as_array()?;
                    let mut lines = vec![
                        "📚 最近日报".to_string(),
                        OUTPUT_DIVIDER.to_string(),
                        format!("设备：{}", device.name),
                    ];
                    if dates.is_empty() {
                        lines.push("暂无日报记录".to_string());
                        return Some(lines.join("\n"));
                    }
                    let items: Vec<String> = dates
                        .iter()
                        .enumerate()
                        .map(|(i, d)| format!("{}. {}", i + 1, d.as_str().unwrap_or("-")))
                        .collect();
                    lines.extend(items);
                    Some(lines.join("\n"))
                }
                None => Some(connection_failed_reply(&device.name)),
            }
        }
        "日报" | "report" => {
            let date = crate::commands::resolve_single_date(parts.get(1).copied());
            let device = match find_device(devices, parts.get(2).copied().unwrap_or(""))
                .or_else(|| devices.first())
            {
                Some(d) => d,
                None => return Some(no_available_device_reply()),
            };
            let url = format!("{}/v1/reports/{}?token={}", device.url, date, device.token);
            match api_get(client, &url).await {
                Some(data) => {
                    if let Some(err) = data.get("error") {
                        return Some(format!(
                            "❌ 查询失败\n设备：{}\n日期：{}\n原因：{}",
                            device.name,
                            date,
                            err.as_str().unwrap_or("未知错误")
                        ));
                    }
                    let content = data.get("content")?.as_str()?.to_string();
                    let content = normalize_report_for_chat(&content);
                    Some(format!(
                        "📄 日报详情\n{OUTPUT_DIVIDER}\n设备：{}\n日期：{}\n\n{}",
                        device.name,
                        date,
                        truncate(&content, 3900)
                    ))
                }
                None => Some(connection_failed_reply(&device.name)),
            }
        }
        "生成日报" | "generate" => {
            let date = crate::commands::resolve_single_date(parts.get(1).copied());
            let device = match find_device(devices, parts.get(2).copied().unwrap_or(""))
                .or_else(|| devices.first())
            {
                Some(d) => d,
                None => return Some(no_available_device_reply()),
            };
            let url = format!("{}/v1/reports/generate?token={}", device.url, device.token);
            match client
                .post(&url)
                .json(&serde_json::json!({"date": date}))
                .timeout(Duration::from_secs(120))
                .send()
                .await
            {
                Ok(r) => {
                    let data: serde_json::Value = r.json().await.ok()?;
                    if let Some(err) = data.get("error") {
                        return Some(format!(
                            "❌ 生成失败\n设备：{}\n日期：{}\n原因：{}",
                            device.name,
                            date,
                            err.as_str().unwrap_or("未知错误")
                        ));
                    }
                    let content = data.get("content")?.as_str()?.to_string();
                    let content = normalize_report_for_chat(&content);
                    Some(format!(
                        "✅ 生成完成\n{OUTPUT_DIVIDER}\n设备：{}\n日期：{}\n\n{}",
                        device.name,
                        date,
                        truncate(&content, 3800)
                    ))
                }
                Err(e) => Some(format!(
                    "❌ 生成失败\n设备：{}\n日期：{}\n原因：{}",
                    device.name, date, e
                )),
            }
        }
        _ => Some(UNKNOWN_CMD_REPLY.to_string()),
    }
}

pub async fn handle_feishu_webhook(
    body: &str,
    config: &AppConfig,
    data_dir: &Path,
) -> FeishuResponse {
    let event: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return FeishuResponse::error(400, format!("JSON parse error: {e}")),
    };

    // URL verification challenge
    if event.get("type").and_then(|v| v.as_str()) == Some("url_verification") {
        let challenge = event
            .get("challenge")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        // Verify token if configured
        if let Some(expected) = config.feishu_verification_token.as_deref() {
            if !expected.is_empty() {
                let token = event.get("token").and_then(|v| v.as_str()).unwrap_or("");
                if token != expected {
                    return FeishuResponse::error(403, "verification token mismatch");
                }
            }
        }
        return FeishuResponse::json(200, &serde_json::json!({"challenge": challenge}));
    }

    // Message event
    let header = match event.get("header") {
        Some(h) => h,
        None => return FeishuResponse::error(400, "missing header"),
    };

    if header.get("event_type").and_then(|v| v.as_str()) != Some("im.message.receive_v1") {
        return FeishuResponse::json(
            200,
            &status_payload("ignored", "event_type_not_supported", None),
        );
    }

    // Verify token in header
    if let Some(expected) = config.feishu_verification_token.as_deref() {
        if !expected.is_empty() {
            let token = header.get("token").and_then(|v| v.as_str()).unwrap_or("");
            if token != expected {
                return FeishuResponse::error(403, "token mismatch");
            }
        }
    }

    let event_body = match event.get("event") {
        Some(b) => b,
        None => return FeishuResponse::error(400, "missing event body"),
    };

    let message = match event_body.get("message") {
        Some(m) => m,
        None => return FeishuResponse::error(400, "missing message"),
    };

    let message_id = match message.get("message_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return FeishuResponse::error(400, "missing message_id"),
    };

    // Only handle text messages
    let msg_type = message
        .get("message_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if msg_type != "text" {
        let reply = NON_TEXT_REPLY.to_string();
        let app_id = match config.feishu_app_id.as_deref() {
            Some(id) if !id.is_empty() => id,
            _ => {
                return FeishuResponse::json(
                    200,
                    &status_payload("ignored", "non_text_message", Some("feishu_app_id 未配置")),
                )
            }
        };
        let app_secret = match config.feishu_app_secret.as_deref() {
            Some(s) if !s.is_empty() => s,
            _ => {
                return FeishuResponse::json(
                    200,
                    &status_payload(
                        "ignored",
                        "non_text_message",
                        Some("feishu_app_secret 未配置"),
                    ),
                )
            }
        };
        let client = match Client::builder().timeout(Duration::from_secs(35)).build() {
            Ok(c) => c,
            Err(e) => return FeishuResponse::error(500, format!("HTTP client error: {e}")),
        };
        if let Some(tenant_token) = get_tenant_token(&client, app_id, app_secret).await {
            let _ = reply_message(&client, &tenant_token, message_id, &reply).await;
        }
        return FeishuResponse::json(
            200,
            &status_payload("ok", "non_text_replied", Some("已提示使用 /help")),
        );
    }

    let content_str = message
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("{}");
    let content: serde_json::Value = serde_json::from_str(content_str).unwrap_or_default();
    let text = content
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    if text.is_empty() {
        return FeishuResponse::json(200, &status_payload("ignored", "empty_text", None));
    }

    let app_id = match config.feishu_app_id.as_deref() {
        Some(id) if !id.is_empty() => id,
        _ => return FeishuResponse::error(500, "feishu_app_id not configured"),
    };
    let app_secret = match config.feishu_app_secret.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => return FeishuResponse::error(500, "feishu_app_secret not configured"),
    };

    let devices = build_device_list(config, data_dir);
    let client = match Client::builder().timeout(Duration::from_secs(35)).build() {
        Ok(c) => c,
        Err(e) => return FeishuResponse::error(500, format!("HTTP client error: {e}")),
    };

    let command = normalize_command(text.split_whitespace().next().unwrap_or(""));
    if let Some(progress) = progress_text_for_command(&command) {
        if let Some(token) = get_tenant_token(&client, app_id, app_secret).await {
            let _ = reply_message(&client, &token, message_id, progress).await;
        }
    }

    let reply = handle_cmd(&client, &devices, text)
        .await
        .unwrap_or_else(|| UNKNOWN_CMD_REPLY.to_string());

    // 始终重新获取 token，避免 handle_cmd 执行过程中旧 token 过期
    let tenant_token = match get_tenant_token(&client, app_id, app_secret).await {
        Some(t) => t,
        None => return FeishuResponse::error(500, "failed to get tenant_access_token"),
    };

    match reply_message(&client, &tenant_token, message_id, &reply).await {
        Some(_) => FeishuResponse::json(
            200,
            &status_payload("ok", "replied", Some("已发送回复消息")),
        ),
        None => FeishuResponse::error(500, "failed to send reply"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_command, normalize_report_for_chat, progress_text_for_command, truncate,
    };

    #[test]
    fn 飞书命令应支持斜杠和机器人后缀() {
        assert_eq!(normalize_command("/help"), "help");
        assert_eq!(normalize_command("/reports@work_review_bot"), "reports");
        assert_eq!(normalize_command("帮助"), "帮助");
    }

    #[test]
    fn 飞书中文截断不应触发utf8边界panic() {
        let content = "# 工作日报\n\n整体进展顺利";
        let got = truncate(content, 8);
        assert_eq!(got.chars().count(), 8);
        assert!(got.ends_with('…'));
    }

    #[test]
    fn 飞书报告格式应在聊天中转为条目文本() {
        let source = "## 一、今日概览\n| 指标 | 数值 |\n|:--|--:|\n| 总工作时长 | 3小时 |\n";
        let rendered = normalize_report_for_chat(source);
        assert!(rendered.contains("一、今日概览"));
        assert!(rendered.contains("总工作时长：3小时"));
        assert!(!rendered.contains("| 指标 |"));
    }

    #[test]
    fn 飞书查询命令应有处理中提示() {
        assert!(progress_text_for_command("reports").is_some());
        assert!(progress_text_for_command("generate").is_some());
        assert!(progress_text_for_command("help").is_none());
    }
}
