use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use work_review_core::config::AppConfig;
use work_review_core::database::Database;
use work_review_core::policy::{CallSource, Permission, PolicyDecision, PolicyEnforcer};
use work_review_skills_engine::engine::SkillEngine;
use work_review_skills_engine::executor::{ExecutionContext, OutputContentType};
use work_review_skills_engine::model::Permission as SkillPermission;

struct AppState {
    db: Database,
    config: AppConfig,
    policy: PolicyEnforcer,
    skills: SkillEngine,
}

fn main() {
    env_logger::init();

    let db_path = std::env::var("WORK_REVIEW_DB_PATH").unwrap_or_else(|_| {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("work-review");
        data_dir
            .join("work_review.db")
            .to_string_lossy()
            .to_string()
    });

    let config_path = std::env::var("WORK_REVIEW_CONFIG_PATH").unwrap_or_else(|_| {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("work-review");
        data_dir.join("config.json").to_string_lossy().to_string()
    });

    let db = match Database::new(std::path::Path::new(&db_path)) {
        Ok(db) => db,
        Err(e) => {
            log::error!("无法打开数据库 {}: {}", db_path, e);
            std::process::exit(1);
        }
    };

    let config = AppConfig::load(std::path::Path::new(&config_path)).unwrap_or_default();

    let mut policy = PolicyEnforcer::new(&config);
    let mut skills = SkillEngine::new();

    // 注册所有内置技能的权限到策略层
    for pkg in skills.list_skills() {
        let perms: Vec<Permission> = pkg
            .required_permissions
            .iter()
            .filter_map(|p| {
                Some(match p {
                    SkillPermission::ReadActivities => Permission::ReadActivities,
                    SkillPermission::ReadReports => Permission::ReadReports,
                    SkillPermission::ReadStats => Permission::ReadStats,
                    SkillPermission::ReadSessions => Permission::ReadSessions,
                    SkillPermission::ReadConfig => Permission::ReadConfig,
                    SkillPermission::WriteReport => Permission::WriteReport,
                    SkillPermission::WriteConfig => Permission::WriteConfig,
                    SkillPermission::ExecuteAi => Permission::ExecuteAi,
                    SkillPermission::ReadDeviceStatus => Permission::ReadDeviceStatus,
                })
            })
            .collect();
        policy.register_skill_permissions(&pkg.id, perms);
    }

    let state = Arc::new(Mutex::new(AppState {
        db,
        config,
        policy,
        skills,
    }));

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                if let Ok(request) = serde_json::from_str::<Value>(&line) {
                    let response = handle_request(&request, &state);
                    if let Ok(output) = serde_json::to_string(&response) {
                        let _ = writeln!(stdout, "{}", output);
                        let _ = stdout.flush();
                    }
                }
            }
            Err(_) => break,
        }
    }
}

fn handle_request(request: &Value, state: &Arc<Mutex<AppState>>) -> Value {
    let method = request["method"].as_str().unwrap_or("");
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let params = request.get("params").cloned().unwrap_or(json!({}));

    match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false },
                    "prompts": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "work-review-mcp-server",
                    "version": "0.1.0"
                }
            }
        }),
        "notifications/initialized" => json!({ "jsonrpc": "2.0" }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tools_list() }
        }),
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("").to_string();
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let result = handle_tool_call(&tool_name, &arguments, state);
            json!({ "jsonrpc": "2.0", "id": id, "result": result })
        }
        "resources/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "resources": resources_list() }
        }),
        "resources/read" => {
            let uri = params["uri"].as_str().unwrap_or("").to_string();
            let result = handle_resource_read(&uri, state);
            json!({ "jsonrpc": "2.0", "id": id, "result": result })
        }
        "prompts/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "prompts": prompts_list() }
        }),
        "prompts/get" => {
            let name = params["name"].as_str().unwrap_or("").to_string();
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let result = handle_prompt_get(&name, &arguments);
            json!({ "jsonrpc": "2.0", "id": id, "result": result })
        }
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("Method not found: {}", method) }
        }),
    }
}

fn tools_list() -> Vec<Value> {
    vec![
        json!({
            "name": "query_timeline",
            "description": "查询指定日期的活动时间线",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "日期，格式 YYYY-MM-DD" },
                    "limit": { "type": "integer", "description": "返回数量限制" },
                    "offset": { "type": "integer", "description": "偏移量" }
                },
                "required": ["date"]
            }
        }),
        json!({
            "name": "get_daily_stats",
            "description": "获取指定日期的工作统计数据",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "日期，格式 YYYY-MM-DD" }
                },
                "required": ["date"]
            }
        }),
        json!({
            "name": "search_activities",
            "description": "搜索工作活动记录",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "搜索关键词" },
                    "date_from": { "type": "string", "description": "起始日期" },
                    "date_to": { "type": "string", "description": "结束日期" },
                    "limit": { "type": "integer", "description": "返回数量限制" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "get_work_sessions",
            "description": "获取指定日期的工作会话分析",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "日期，格式 YYYY-MM-DD" }
                },
                "required": ["date"]
            }
        }),
        json!({
            "name": "analyze_intents",
            "description": "分析指定日期的工作意图",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "日期，格式 YYYY-MM-DD" }
                },
                "required": ["date"]
            }
        }),
        json!({
            "name": "generate_report",
            "description": "生成指定日期的工作日报",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "日期，格式 YYYY-MM-DD" },
                    "locale": { "type": "string", "description": "语言，zh-CN/en/zh-TW" }
                },
                "required": ["date"]
            }
        }),
        json!({
            "name": "get_report",
            "description": "获取已生成的日报",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "日期，格式 YYYY-MM-DD" },
                    "locale": { "type": "string", "description": "语言" }
                },
                "required": ["date"]
            }
        }),
        json!({
            "name": "get_device_status",
            "description": "获取当前设备状态信息",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "execute_skill",
            "description": "执行指定的 Skills Engine 技能",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "skill_id": { "type": "string", "description": "技能 ID" },
                    "params": { "type": "object", "description": "技能参数" }
                },
                "required": ["skill_id"]
            }
        }),
        json!({
            "name": "list_skills",
            "description": "列出所有可用技能",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "get_skill_stats",
            "description": "获取技能执行统计",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "skill_id": { "type": "string", "description": "技能 ID，不传则返回所有" }
                }
            }
        }),
    ]
}

fn with_policy_check<F>(
    state: &Arc<Mutex<AppState>>,
    tool_name: &str,
    permission: Permission,
    f: F,
) -> Value
where
    F: FnOnce(&mut AppState) -> Value,
{
    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
    let source = CallSource::McpTool {
        tool_name: tool_name.to_string(),
        client_id: None,
    };
    match s.policy.check_permission(&source, permission) {
        PolicyDecision::Allow => f(&mut s),
        PolicyDecision::AllowSanitized => {
            let mut result = f(&mut s);
            // 对 result 中的敏感字段做脱敏
            if let Some(content) = result.get_mut("content") {
                if let Some(arr) = content.as_array_mut() {
                    for item in arr.iter_mut() {
                        if let Some(text) = item.get_mut("text") {
                            // 简单脱敏：移除 screenshot_path 字段
                            if let Ok(mut v) = serde_json::from_value::<Value>(text.clone()) {
                                if let Some(arr) = v.as_array_mut() {
                                    for item in arr.iter_mut() {
                                        if let Some(obj) = item.as_object_mut() {
                                            obj.remove("screenshot_path");
                                        }
                                    }
                                }
                                *text = serde_json::to_string_pretty(&v).unwrap_or_default().into();
                            }
                        }
                    }
                }
            }
            result
        }
        PolicyDecision::Deny => tool_error(&format!("权限被拒绝: 无 {:?} 权限", permission)),
    }
}

fn handle_tool_call(name: &str, args: &Value, state: &Arc<Mutex<AppState>>) -> Value {
    match name {
        "query_timeline" => with_policy_check(state, name, Permission::ReadActivities, |s| {
            let date = args["date"].as_str().unwrap_or("");
            let limit = args["limit"].as_u64().map(|l| l as u32);
            let offset = args["offset"].as_u64().map(|o| o as u32);
            match s.db.get_timeline(date, limit, offset) {
                Ok(activities) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&activities).unwrap_or_default() }]
                }),
                Err(e) => tool_error(&format!("查询时间线失败: {e}")),
            }
        }),
        "get_daily_stats" => with_policy_check(state, name, Permission::ReadStats, |s| {
            let date = args["date"].as_str().unwrap_or("");
            let segments = s.config.effective_work_segments();
            match s.db.get_daily_stats_with_segments(date, &segments) {
                Ok(stats) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&stats).unwrap_or_default() }]
                }),
                Err(e) => tool_error(&format!("获取统计失败: {e}")),
            }
        }),
        "search_activities" => with_policy_check(state, name, Permission::ReadActivities, |s| {
            let query = args["query"].as_str().unwrap_or("");
            let date_from = args["date_from"].as_str();
            let date_to = args["date_to"].as_str();
            let limit = args["limit"].as_u64().unwrap_or(50) as usize;
            match s.db.search_memory(query, date_from, date_to, limit) {
                Ok(results) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string_pretty(&results).unwrap_or_default() }]
                }),
                Err(e) => tool_error(&format!("搜索失败: {e}")),
            }
        }),
        "get_work_sessions" => with_policy_check(state, name, Permission::ReadSessions, |s| {
            let date = args["date"].as_str().unwrap_or("");
            match s.db.get_timeline(date, None, None) {
                Ok(activities) => {
                    let sessions =
                        work_review_core::work_intelligence::build_work_sessions(&activities);
                    json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&sessions).unwrap_or_default() }]
                    })
                }
                Err(e) => tool_error(&format!("获取会话失败: {e}")),
            }
        }),
        "analyze_intents" => with_policy_check(state, name, Permission::ReadSessions, |s| {
            let date = args["date"].as_str().unwrap_or("");
            match s.db.get_timeline(date, None, None) {
                Ok(activities) => {
                    let intents = work_review_core::work_intelligence::analyze_intents(&activities);
                    json!({
                        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&intents.summary).unwrap_or_default() }]
                    })
                }
                Err(e) => tool_error(&format!("意图分析失败: {e}")),
            }
        }),
        "generate_report" => with_policy_check(state, name, Permission::WriteReport, |s| {
            let date = args["date"].as_str().unwrap_or("");
            match s.db.get_daily_stats(date) {
                Ok(stats) => {
                    let summary = work_review_core::analysis::generate_stats_summary(&stats);
                    json!({
                        "content": [{ "type": "text", "text": format!("工作日报 - {}\n\n{}", date, summary) }]
                    })
                }
                Err(e) => tool_error(&format!("生成报告失败: {e}")),
            }
        }),
        "get_report" => with_policy_check(state, name, Permission::ReadReports, |s| {
            let date = args["date"].as_str().unwrap_or("");
            let locale = args["locale"].as_str();
            match s.db.get_report(date, locale) {
                Ok(Some(report)) => json!({
                    "content": [{ "type": "text", "text": report.content }]
                }),
                Ok(None) => tool_error(&format!("未找到 {} 的报告", date)),
                Err(e) => tool_error(&format!("获取报告失败: {e}")),
            }
        }),
        "get_device_status" => with_policy_check(state, name, Permission::ReadDeviceStatus, |s| {
            let audit_stats = s.policy.get_call_stats();
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&json!({
                        "status": "running",
                        "platform": std::env::consts::OS,
                        "arch": std::env::consts::ARCH,
                        "version": env!("CARGO_PKG_VERSION"),
                        "skills_count": s.skills.list_skills().len(),
                        "audit_summary": audit_stats,
                    })).unwrap_or_default()
                }]
            })
        }),
        "execute_skill" => with_policy_check(state, name, Permission::ExecuteSkill, |s| {
            let skill_id = args["skill_id"].as_str().unwrap_or("").to_string();
            let params_map: HashMap<String, Value> = args
                .get("params")
                .and_then(|p| serde_json::from_value(p.clone()).ok())
                .unwrap_or_default();

            let ctx = ExecutionContext {
                params: params_map,
                db_path: String::new(),
                ai_endpoint: Some(s.config.text_model.endpoint.clone()),
                ai_api_key: s.config.text_model.api_key.clone(),
                ai_model: Some(s.config.text_model.model.clone()),
            };

            let result = s.skills.execute(&skill_id, &ctx);
            let content_type = match result.content_type {
                OutputContentType::Text => "text",
                OutputContentType::Markdown => "markdown",
                OutputContentType::Json => "json",
            };
            json!({
                "content": [{
                    "type": "text",
                    "text": format!("Skill: {} | Type: {} | Duration: {}ms | Success: {}\n\n{}",
                        result.skill_id, content_type, result.duration_ms, result.success,
                        result.output)
                }],
                "isError": !result.success
            })
        }),
        "list_skills" => with_policy_check(state, name, Permission::ExecuteSkill, |s| {
            let skills: Vec<Value> = s
                .skills
                .list_skills()
                .iter()
                .map(|pkg| {
                    json!({
                        "id": pkg.id,
                        "name": pkg.name,
                        "description": pkg.description,
                        "category": format!("{:?}", pkg.category),
                        "enabled": pkg.enabled,
                        "version": pkg.version,
                        "adaptive_enabled": pkg.adaptive.enabled,
                    })
                })
                .collect();
            json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&skills).unwrap_or_default() }]
            })
        }),
        "get_skill_stats" => with_policy_check(state, name, Permission::ExecuteSkill, |s| {
            let skill_id = args["skill_id"].as_str();
            let stats = if let Some(id) = skill_id {
                match s.skills.get_skill_state(id) {
                    Some(state) => vec![(id, &state.stats)],
                    None => return tool_error(&format!("技能未找到: {}", id)),
                }
            } else {
                s.skills.get_all_stats()
            };
            let stats_json: Vec<Value> = stats
                .iter()
                .map(|(id, stat)| {
                    json!({
                        "skill_id": id,
                        "total_executions": stat.total_executions,
                        "success_count": stat.success_count,
                        "failure_count": stat.failure_count,
                        "avg_duration_ms": stat.avg_duration_ms,
                        "last_executed_at": stat.last_executed_at,
                    })
                })
                .collect();
            json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&stats_json).unwrap_or_default() }]
            })
        }),
        _ => tool_error(&format!("未知工具: {name}")),
    }
}

fn resources_list() -> Vec<Value> {
    vec![
        json!({ "uri": "timeline/today", "name": "今日时间线", "description": "获取今天的活动时间线", "mimeType": "application/json" }),
        json!({ "uri": "sessions/current", "name": "当前工作会话", "description": "获取当前进行中的工作会话", "mimeType": "application/json" }),
        json!({ "uri": "stats/weekly", "name": "本周统计", "description": "获取本周工作统计数据", "mimeType": "application/json" }),
    ]
}

fn handle_resource_read(uri: &str, state: &Arc<Mutex<AppState>>) -> Value {
    let s = state.lock().unwrap_or_else(|e| e.into_inner());
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    match uri {
        "timeline/today" => match s.db.get_timeline(&today, Some(50), None) {
            Ok(activities) => resource_result(
                uri,
                &serde_json::to_string_pretty(&activities).unwrap_or_default(),
            ),
            Err(e) => resource_result(uri, &format!("Error: {e}")),
        },
        "sessions/current" => match s.db.get_timeline(&today, None, None) {
            Ok(activities) => {
                let sessions =
                    work_review_core::work_intelligence::build_work_sessions(&activities);
                resource_result(
                    uri,
                    &serde_json::to_string_pretty(&sessions).unwrap_or_default(),
                )
            }
            Err(e) => resource_result(uri, &format!("Error: {e}")),
        },
        "stats/weekly" => {
            let mut weekly = Vec::new();
            for i in 0..7 {
                let date = (chrono::Local::now() - chrono::Duration::days(i))
                    .format("%Y-%m-%d")
                    .to_string();
                if let Ok(stats) = s.db.get_daily_stats(&date) {
                    weekly.push(json!({ "date": date, "total_duration": stats.total_duration, "screenshot_count": stats.screenshot_count }));
                }
            }
            resource_result(
                uri,
                &serde_json::to_string_pretty(&weekly).unwrap_or_default(),
            )
        }
        _ => resource_result(uri, &format!("Unknown resource: {uri}")),
    }
}

fn resource_result(uri: &str, text: &str) -> Value {
    json!({
        "contents": [{ "uri": uri, "mimeType": "application/json", "text": text }]
    })
}

fn prompts_list() -> Vec<Value> {
    vec![
        json!({ "name": "daily_review", "description": "每日工作回顾提示词", "arguments": [{ "name": "date", "description": "回顾日期，默认今天", "required": false }] }),
        json!({ "name": "weekly_summary", "description": "每周工作总结提示词", "arguments": [{ "name": "week_start", "description": "周一开始日期", "required": false }] }),
        json!({ "name": "project_time_audit", "description": "项目时间审计提示词", "arguments": [{ "name": "project", "description": "项目关键词", "required": true }] }),
    ]
}

fn handle_prompt_get(name: &str, args: &Value) -> Value {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let (desc, text) = match name {
        "daily_review" => {
            let date = args["date"].as_str().unwrap_or(&today);
            ("每日工作回顾", format!("请帮我回顾 {} 的工作情况。先使用 get_daily_stats 获取统计数据，然后使用 query_timeline 获取时间线，最后给出今日工作总结和改进建议。", date))
        }
        "weekly_summary" => {
            let week_start = args["week_start"].as_str().unwrap_or(&today);
            ("每周工作总结", format!("请帮我总结从 {} 开始的这一周的工作。逐日获取统计数据，分析工作模式、效率变化，并给出下周建议。", week_start))
        }
        "project_time_audit" => {
            let project = args["project"].as_str().unwrap_or("");
            ("项目时间审计", format!("请帮我审计项目「{}」的时间投入。使用 search_activities 搜索相关活动，分析时间分布和效率，给出时间管理建议。", project))
        }
        _ => ("未知提示词", String::new()),
    };
    json!({
        "description": desc,
        "messages": [{ "role": "user", "content": { "type": "text", "text": text } }]
    })
}

fn tool_error(message: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}
