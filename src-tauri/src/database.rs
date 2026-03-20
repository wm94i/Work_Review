use crate::error::{AppError, Result};
use chrono::{Local, MappedLocalTime, NaiveDateTime, TimeZone};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// 安全地将 NaiveDateTime 转换为本地时间戳
/// 在 DST 跳变时不会 panic：
/// - Ambiguous（秋季回拨）：取较早的时间
/// - None（春季前跳）：向前偏移1小时后重试
fn safe_local_timestamp(ndt: NaiveDateTime) -> i64 {
    match Local.from_local_datetime(&ndt) {
        MappedLocalTime::Single(dt) => dt.timestamp(),
        MappedLocalTime::Ambiguous(dt, _) => dt.timestamp(),
        MappedLocalTime::None => {
            // DST 跳变导致该本地时间不存在，向前偏移1小时
            let shifted = ndt + chrono::Duration::hours(1);
            Local
                .from_local_datetime(&shifted)
                .earliest()
                .map(|dt| dt.timestamp())
                .unwrap_or_else(|| ndt.and_utc().timestamp())
        }
    }
}

/// 活动记录
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Activity {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub app_name: String,
    pub window_title: String,
    pub screenshot_path: String,
    pub ocr_text: Option<String>,
    pub category: String,
    pub duration: i64,
    /// 浏览器 URL（如果当前是浏览器应用）
    #[serde(default)]
    pub browser_url: Option<String>,
}

/// 每日报告
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DailyReport {
    pub date: String,
    pub content: String,
    pub ai_mode: String,
    pub model_name: Option<String>,
    pub created_at: i64,
}

/// 应用使用统计
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppUsage {
    pub app_name: String,
    pub duration: i64,
    pub count: i64,
}

/// 分类使用统计
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoryUsage {
    pub category: String,
    pub duration: i64,
}

/// 小时摘要
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HourlySummary {
    pub id: Option<i64>,
    /// 日期 YYYY-MM-DD
    pub date: String,
    /// 小时 (0-23)
    pub hour: i32,
    /// AI 生成的摘要内容
    pub summary: String,
    /// 该小时的主要应用
    pub main_apps: String,
    /// 该小时的活动数量
    pub activity_count: i32,
    /// 该小时的总时长（秒）
    pub total_duration: i64,
    /// 代表性截图路径列表（JSON数组）
    pub representative_screenshots: Option<String>,
    /// 创建时间
    pub created_at: i64,
}

/// 每日统计
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DailyStats {
    pub total_duration: i64,
    pub screenshot_count: i64,
    pub app_usage: Vec<AppUsage>,
    pub category_usage: Vec<CategoryUsage>,
    pub browser_duration: i64,
    pub url_usage: Vec<UrlUsage>,
    pub domain_usage: Vec<DomainUsage>,
    /// 按浏览器分组的使用统计
    pub browser_usage: Vec<BrowserUsage>,
    /// 工作时间内的活动时长（新增）
    #[serde(default)]
    pub work_time_duration: i64,
}

/// 域名使用统计（按域名分组）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DomainUsage {
    pub domain: String,
    pub duration: i64,
    pub urls: Vec<UrlDetail>,
}

/// URL 详情
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlDetail {
    pub url: String,
    pub duration: i64,
}

/// 浏览器使用统计（按浏览器应用分组）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowserUsage {
    /// 浏览器名称（如 Chrome, Safari, Arc 等）
    pub browser_name: String,
    /// 总使用时长
    pub duration: i64,
    /// 该浏览器下访问的域名列表
    pub domains: Vec<DomainUsage>,
}

/// URL 使用统计（保留兼容）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlUsage {
    pub url: String,
    pub domain: String,
    pub duration: i64,
}

/// 规范化 URL（用于合并判断）
/// 移除末尾斜杠、规范化空白字符
pub fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// 数据库管理器
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// 创建新的数据库连接
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        Ok(db)
    }

    /// 初始化数据库表
    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS activities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                app_name TEXT NOT NULL,
                window_title TEXT NOT NULL,
                screenshot_path TEXT NOT NULL,
                ocr_text TEXT,
                category TEXT NOT NULL,
                duration INTEGER NOT NULL,
                browser_url TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_activities_timestamp_app ON activities (timestamp, app_name)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_reports (
                date TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                ai_mode TEXT NOT NULL,
                model_name TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // 小时摘要表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS hourly_summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                hour INTEGER NOT NULL,
                summary TEXT NOT NULL,
                main_apps TEXT NOT NULL,
                activity_count INTEGER NOT NULL,
                total_duration INTEGER NOT NULL,
                representative_screenshots TEXT,
                created_at INTEGER NOT NULL,
                UNIQUE(date, hour)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hourly_summaries_date ON hourly_summaries (date)",
            [],
        )?;

        // 迁移：添加 browser_url 列（如果不存在）
        let _ = conn.execute("ALTER TABLE activities ADD COLUMN browser_url TEXT", []);

        Ok(())
    }

    /// 插入活动记录
    pub fn insert_activity(&self, activity: &Activity) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let normalized_browser_url = activity
            .browser_url
            .as_deref()
            .map(normalize_url)
            .filter(|url| !url.is_empty());

        conn.execute(
            "INSERT INTO activities (timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                activity.timestamp,
                activity.app_name,
                activity.window_title,
                activity.screenshot_path,
                activity.ocr_text,
                activity.category,
                activity.duration,
                normalized_browser_url,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// 获取指定应用最近24小时内的最新一条活动记录
    pub fn get_last_activity_by_app(&self, app_name: &str) -> Result<Option<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 回溯24小时
        let start_ts = chrono::Local::now().timestamp() - 86400;

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url 
             FROM activities 
             WHERE app_name = ?1 AND timestamp >= ?2
             ORDER BY id DESC 
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![app_name, start_ts])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Activity {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                screenshot_path: row.get(4)?,
                ocr_text: row.get(5)?,
                category: row.get(6)?,
                duration: row.get(7)?,
                browser_url: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 获取指定 URL 最近24小时内的最新一条活动记录
    pub fn get_last_activity_by_url(&self, url: &str) -> Result<Option<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 回溯24小时
        let start_ts = chrono::Local::now().timestamp() - 86400;

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url 
             FROM activities 
             WHERE browser_url = ?1 AND timestamp >= ?2
             ORDER BY id DESC 
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![url, start_ts])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Activity {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                screenshot_path: row.get(4)?,
                ocr_text: row.get(5)?,
                category: row.get(6)?,
                duration: row.get(7)?,
                browser_url: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 获取指定应用今天的最近一条活动记录（用于合并判断）
    pub fn get_latest_activity_by_app(&self, app_name: &str) -> Result<Option<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 获取今天的开始时间戳（当天 00:00:00）
        let today_start = {
            let now = chrono::Local::now();
            let ndt = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            safe_local_timestamp(ndt)
        };

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url
             FROM activities
             WHERE app_name = ?1 AND timestamp >= ?2
             ORDER BY id DESC
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![app_name, today_start])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Activity {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                screenshot_path: row.get(4)?,
                ocr_text: row.get(5)?,
                category: row.get(6)?,
                duration: row.get(7)?,
                browser_url: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 获取指定应用 + 窗口标题今天的最近一条活动记录
    /// 当浏览器 URL 暂时不可用时，用于避免不同标签页互相串时长
    pub fn get_latest_activity_by_app_title(
        &self,
        app_name: &str,
        window_title: &str,
    ) -> Result<Option<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let today_start = {
            let now = chrono::Local::now();
            let ndt = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            safe_local_timestamp(ndt)
        };

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url
             FROM activities
             WHERE app_name = ?1 AND window_title = ?2 AND timestamp >= ?3
             ORDER BY id DESC
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![app_name, window_title, today_start])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Activity {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                screenshot_path: row.get(4)?,
                ocr_text: row.get(5)?,
                category: row.get(6)?,
                duration: row.get(7)?,
                browser_url: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 按 URL 获取今天的活动记录（用于浏览器 URL 合并）
    /// 使用规范化 URL 进行匹配，解决末尾斜杠差异问题
    pub fn get_latest_activity_by_url(&self, browser_url: &str) -> Result<Option<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let today_start = {
            let now = chrono::Local::now();
            let ndt = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
            safe_local_timestamp(ndt)
        };

        // 规范化输入 URL
        let normalized_url = normalize_url(browser_url);
        log::debug!("URL 合并查询: 原始='{browser_url}', 规范化='{normalized_url}'");

        // 使用 RTRIM 规范化数据库中的 URL 进行比较
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url 
             FROM activities 
             WHERE RTRIM(browser_url, '/') = ?1 AND timestamp >= ?2
             ORDER BY id DESC 
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![normalized_url, today_start])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Activity {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                screenshot_path: row.get(4)?,
                ocr_text: row.get(5)?,
                category: row.get(6)?,
                duration: row.get(7)?,
                browser_url: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 根据 ID 获取单个活动
    pub fn get_activity_by_id(&self, id: i64) -> Result<Option<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url 
             FROM activities WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Activity {
                id: Some(row.get(0)?),
                timestamp: row.get(1)?,
                app_name: row.get(2)?,
                window_title: row.get(3)?,
                screenshot_path: row.get(4)?,
                ocr_text: row.get(5)?,
                category: row.get(6)?,
                duration: row.get(7)?,
                browser_url: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 合并活动：累加时长、追加OCR、更新截图路径
    pub fn merge_activity(
        &self,
        id: i64,
        duration_delta: i64,
        new_ocr: Option<&str>,
        new_screenshot_path: &str,
        new_timestamp: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 获取现有的 OCR 内容
        let existing_ocr: Option<String> = conn
            .query_row(
                "SELECT ocr_text FROM activities WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();

        // 合并 OCR：追加新内容
        let merged_ocr = match (existing_ocr, new_ocr) {
            (Some(existing), Some(new)) if !new.is_empty() => {
                // 追加新内容，用分隔符隔开
                Some(format!("{existing}\n---\n{new}"))
            }
            (Some(existing), _) => Some(existing),
            (None, Some(new)) => Some(new.to_string()),
            (None, None) => None,
        };

        conn.execute(
            "UPDATE activities 
             SET duration = duration + ?1, 
                 ocr_text = ?2, 
                 screenshot_path = ?3,
                 timestamp = ?4
             WHERE id = ?5",
            params![
                duration_delta,
                merged_ocr,
                new_screenshot_path,
                new_timestamp,
                id
            ],
        )?;

        Ok(())
    }

    /// 精确增加活动时长（用于事件驱动时长计算）
    /// 当检测到应用切换时，将上一个应用的实际使用时长累加到其记录
    pub fn add_duration(&self, id: i64, duration_delta: i64) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        conn.execute(
            "UPDATE activities SET duration = duration + ?1 WHERE id = ?2",
            params![duration_delta, id],
        )?;

        log::debug!("精确时长累加: id={id}, +{duration_delta}秒");
        Ok(())
    }

    /// 更新活动的 OCR 文本
    pub fn update_activity_ocr(&self, id: i64, ocr_text: Option<String>) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        conn.execute(
            "UPDATE activities SET ocr_text = ?1 WHERE id = ?2",
            params![ocr_text, id],
        )?;

        Ok(())
    }

    /// 删除指定应用在指定时间之后的旧记录（保留 keep_id），返回删除数量和截图路径
    pub fn delete_old_activities_by_app(
        &self,
        app_name: &str,
        keep_id: i64,
        since_timestamp: i64,
    ) -> Result<(usize, Vec<String>)> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 先获取要删除的记录的截图路径
        let mut stmt = conn.prepare(
            "SELECT screenshot_path FROM activities 
             WHERE app_name = ?1 AND id != ?2 AND timestamp >= ?3",
        )?;

        let paths: Vec<String> = stmt
            .query_map(params![app_name, keep_id, since_timestamp], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(|r| r.ok())
            .filter(|p| !p.is_empty())
            .collect();

        // 删除旧记录
        let deleted = conn.execute(
            "DELETE FROM activities 
             WHERE app_name = ?1 AND id != ?2 AND timestamp >= ?3",
            params![app_name, keep_id, since_timestamp],
        )?;
        Ok((deleted, paths))
    }

    /// 清理当天的重复活动记录
    /// 对于每个应用（非浏览器），合并同名记录
    /// 对于浏览器，按 URL 合并记录
    /// 将重复记录的 duration 累加到保留记录后再删除重复项
    /// 返回删除的记录数和截图路径
    pub fn cleanup_duplicate_activities(&self, date: &str) -> Result<(usize, Vec<String>)> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 获取当天的时间戳范围
        let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| AppError::Config(e.to_string()))?;

        let start_ts = safe_local_timestamp(date_parsed.and_hms_opt(0, 0, 0).unwrap());
        let end_ts = start_ts + 86400;

        // 获取当天所有活动
        let mut stmt = conn.prepare(
            "SELECT id, app_name, browser_url, duration FROM activities
             WHERE timestamp >= ?1 AND timestamp < ?2",
        )?;

        let activities: Vec<(i64, String, Option<String>, i64)> = stmt
            .query_map(params![start_ts, end_ts], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 按 app_name 或 browser_url 分组，记录每组所有 id 和 duration
        use std::collections::HashMap;
        // key -> Vec<(id, duration)>
        let mut groups: HashMap<String, Vec<(i64, i64)>> = HashMap::new();

        for (id, app_name, browser_url, duration) in activities {
            let key = if let Some(ref url) = browser_url {
                if !url.is_empty() {
                    format!("url:{url}")
                } else {
                    format!("app:{app_name}")
                }
            } else {
                format!("app:{app_name}")
            };

            groups.entry(key).or_default().push((id, duration));
        }

        let mut total_deleted = 0usize;
        let mut all_paths = Vec::new();

        for (_key, mut entries) in groups {
            // 只有一条记录的组无需清理
            if entries.len() <= 1 {
                continue;
            }

            // 按 duration 降序排列，保留最长的那条
            entries.sort_by(|a, b| b.1.cmp(&a.1));
            let keep_id = entries[0].0;

            // 计算需要累加的 duration（其余记录的总时长）
            let extra_duration: i64 = entries[1..].iter().map(|(_, d)| *d).sum();
            let ids_to_delete: Vec<i64> = entries[1..].iter().map(|(id, _)| *id).collect();

            // 先将额外的 duration 累加到保留记录
            if extra_duration > 0 {
                conn.execute(
                    "UPDATE activities SET duration = duration + ?1 WHERE id = ?2",
                    params![extra_duration, keep_id],
                )?;
            }

            // 获取要删除的记录的截图路径
            for del_id in &ids_to_delete {
                let path: String = conn
                    .query_row(
                        "SELECT screenshot_path FROM activities WHERE id = ?1",
                        params![del_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();
                if !path.is_empty() {
                    all_paths.push(path);
                }
            }

            // 删除重复记录
            for del_id in &ids_to_delete {
                conn.execute("DELETE FROM activities WHERE id = ?1", params![del_id])?;
                total_deleted += 1;
            }
        }

        log::info!("清理重复记录: 删除 {total_deleted} 条，时长已合并到保留记录");

        Ok((total_deleted, all_paths))
    }

    /// 删除指定时间之前的所有活动记录
    pub fn delete_activities_before(&self, before_timestamp: i64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let deleted = conn.execute(
            "DELETE FROM activities WHERE timestamp < ?1",
            params![before_timestamp],
        )?;

        Ok(deleted)
    }

    /// 获取指定日期的统计数据
    /// work_start_hour: 工作开始时间（0-23），默认 9
    /// work_end_hour: 工作结束时间（0-23），默认 18
    pub fn get_daily_stats_with_work_time(
        &self,
        date: &str,
        work_start_hour: u8,
        work_end_hour: u8,
        work_start_minute: u8,
        work_end_minute: u8,
    ) -> Result<DailyStats> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 获取当天的时间戳范围
        let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| AppError::Config(e.to_string()))?;

        let start_ts = safe_local_timestamp(date_parsed.and_hms_opt(0, 0, 0).unwrap());
        let end_ts = start_ts + 86400;

        // 计算工作时间范围的时间戳（clamp 到合法小时范围）
        let ws = (work_start_hour as u32).min(23);
        let we = (work_end_hour as u32).min(23);
        let wsm = (work_start_minute as u32).min(59);
        let wem = (work_end_minute as u32).min(59);
        let work_start_ts = safe_local_timestamp(date_parsed.and_hms_opt(ws, wsm, 0).unwrap());
        let work_end_ts = safe_local_timestamp(date_parsed.and_hms_opt(we, wem, 0).unwrap());

        // 获取总时长和截图数
        let (total_duration, screenshot_count): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0), COUNT(*) FROM activities WHERE timestamp >= ?1 AND timestamp < ?2",
            params![start_ts, end_ts],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // 获取工作时间内的活动时长
        let work_time_duration: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration), 0) FROM activities WHERE timestamp >= ?1 AND timestamp < ?2",
            params![work_start_ts, work_end_ts],
            |row| row.get(0),
        ).unwrap_or(0);

        // 获取应用使用统计
        let mut stmt = conn.prepare(
            "SELECT app_name, SUM(duration) as total_duration, COUNT(*) as count 
             FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 
             GROUP BY app_name 
             ORDER BY total_duration DESC",
        )?;

        let app_usage_rows: Vec<AppUsage> = stmt
            .query_map(params![start_ts, end_ts], |row| {
                Ok(AppUsage {
                    app_name: row.get(0)?,
                    duration: row.get(1)?,
                    count: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 在 Rust 侧按显示名再次聚合，避免 work-review / Work Review 等别名被拆成多条
        let mut app_usage_map: std::collections::HashMap<String, AppUsage> =
            std::collections::HashMap::new();
        for usage in app_usage_rows {
            let normalized_name = crate::monitor::normalize_display_app_name(&usage.app_name);
            let entry = app_usage_map.entry(normalized_name.clone()).or_insert(AppUsage {
                app_name: normalized_name,
                duration: 0,
                count: 0,
            });
            entry.duration += usage.duration;
            entry.count += usage.count;
        }

        let mut app_usage: Vec<AppUsage> = app_usage_map.into_values().collect();
        app_usage.sort_by(|a, b| {
            b.duration
                .cmp(&a.duration)
                .then_with(|| b.count.cmp(&a.count))
                .then_with(|| a.app_name.cmp(&b.app_name))
        });

        // 获取分类使用统计
        let mut stmt = conn.prepare(
            "SELECT category, SUM(duration) as total_duration 
             FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 
             GROUP BY category 
             ORDER BY total_duration DESC",
        )?;

        let category_usage: Vec<CategoryUsage> = stmt
            .query_map(params![start_ts, end_ts], |row| {
                Ok(CategoryUsage {
                    category: row.get(0)?,
                    duration: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 计算浏览器总时长
        let browser_duration: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(duration), 0) FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 AND category = 'browser'",
                params![start_ts, end_ts],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // 计算各 URL 使用时长（按浏览器和URL分组）
        let mut browser_url_stmt = conn.prepare(
            "SELECT app_name, RTRIM(browser_url, '/') as browser_url, SUM(duration) as total_duration 
             FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 
               AND browser_url IS NOT NULL AND browser_url != ''
               AND category = 'browser'
             GROUP BY app_name, RTRIM(browser_url, '/') 
             ORDER BY app_name, total_duration DESC",
        )?;

        // 收集所有浏览器的 URL 数据
        let browser_url_rows: Vec<(String, String, i64)> = browser_url_stmt
            .query_map(params![start_ts, end_ts], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 按浏览器 -> 域名 -> URL 三级结构组织数据
        // 结构: { browser_name: { domain: { url: duration } } }
        let mut browser_map: std::collections::HashMap<
            String,
            std::collections::HashMap<String, Vec<UrlDetail>>,
        > = std::collections::HashMap::new();

        for (app_name, url, duration) in &browser_url_rows {
            let normalized_browser_name = crate::monitor::normalize_display_app_name(app_name);
            // 提取域名
            let domain = url.split('/').nth(2).unwrap_or(url).to_string();

            let domain_map = browser_map.entry(normalized_browser_name).or_default();
            let url_list = domain_map.entry(domain).or_default();
            url_list.push(UrlDetail {
                url: url.clone(),
                duration: *duration,
            });
        }

        // 获取各浏览器的总时长
        let mut browser_duration_stmt = conn.prepare(
            "SELECT app_name, SUM(duration) as total_duration 
             FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 AND category = 'browser'
             GROUP BY app_name 
             ORDER BY total_duration DESC",
        )?;

        let browser_duration_rows: Vec<(String, i64)> = browser_duration_stmt
            .query_map(params![start_ts, end_ts], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut browser_duration_map: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        for (raw_browser_name, total_duration) in browser_duration_rows {
            let normalized_browser_name =
                crate::monitor::normalize_display_app_name(&raw_browser_name);
            *browser_duration_map
                .entry(normalized_browser_name)
                .or_insert(0) += total_duration;
        }

        let browser_durations: Vec<(String, i64)> = browser_duration_map.into_iter().collect();

        // 构建 BrowserUsage 列表
        let mut browser_usage: Vec<BrowserUsage> = browser_durations
            .iter()
            .map(|(browser_name, total_duration)| {
                // 获取该浏览器下的域名统计
                let domain_map = browser_map.get(browser_name);
                let mut domains: Vec<DomainUsage> = match domain_map {
                    Some(dm) => dm
                        .iter()
                        .map(|(domain, urls)| {
                            let domain_duration: i64 = urls.iter().map(|u| u.duration).sum();
                            DomainUsage {
                                domain: domain.clone(),
                                duration: domain_duration,
                                urls: urls.clone(),
                            }
                        })
                        .collect(),
                    None => Vec::new(),
                };
                // 按时长排序域名
                domains.sort_by(|a, b| b.duration.cmp(&a.duration));

                BrowserUsage {
                    browser_name: browser_name.clone(),
                    duration: *total_duration,
                    domains,
                }
            })
            .collect();

        // 按时长排序浏览器
        browser_usage.sort_by(|a, b| b.duration.cmp(&a.duration));

        // 兼容旧的 url_usage 和 domain_usage（保持向后兼容）
        let mut url_stmt = conn.prepare(
            "SELECT RTRIM(browser_url, '/') as browser_url, SUM(duration) as total_duration 
             FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 AND browser_url IS NOT NULL AND browser_url != ''
             GROUP BY RTRIM(browser_url, '/') 
             ORDER BY total_duration DESC
             LIMIT 10"
        )?;

        let url_usage: Vec<UrlUsage> = url_stmt
            .query_map(params![start_ts, end_ts], |row| {
                let url: String = row.get(0)?;
                let domain = url.split('/').nth(2).unwrap_or(&url).to_string();
                Ok(UrlUsage {
                    url,
                    domain,
                    duration: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        // 按域名分组统计（兼容旧逻辑）
        let mut domain_map_compat: std::collections::HashMap<String, (i64, Vec<UrlDetail>)> =
            std::collections::HashMap::new();
        for u in &url_usage {
            let entry = domain_map_compat
                .entry(u.domain.clone())
                .or_insert((0, Vec::new()));
            entry.0 += u.duration;
            entry.1.push(UrlDetail {
                url: u.url.clone(),
                duration: u.duration,
            });
        }

        let mut domain_usage: Vec<DomainUsage> = domain_map_compat
            .into_iter()
            .map(|(domain, (duration, urls))| DomainUsage {
                domain,
                duration,
                urls,
            })
            .collect();
        domain_usage.sort_by(|a, b| b.duration.cmp(&a.duration));
        domain_usage.truncate(10);

        Ok(DailyStats {
            total_duration,
            screenshot_count,
            app_usage,
            category_usage,
            browser_duration,
            url_usage,
            domain_usage,
            browser_usage,
            work_time_duration,
        })
    }

    /// 获取指定日期的统计数据（使用默认工作时间 9:00-18:00）
    pub fn get_daily_stats(&self, date: &str) -> Result<DailyStats> {
        self.get_daily_stats_with_work_time(date, 9, 18, 0, 0)
    }

    /// 获取指定日期的时间线 (支持分页)
    /// 使用 GROUP BY 聚合，确保同一应用（同 URL）只返回一条记录
    pub fn get_timeline(
        &self,
        date: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| AppError::Config(e.to_string()))?;
        let start_ts = safe_local_timestamp(date_parsed.and_hms_opt(0, 0, 0).unwrap());
        let end_ts = start_ts + 86400;

        let limit_val = limit.unwrap_or(1000);
        let offset_val = offset.unwrap_or(0);

        let mut stmt = conn.prepare(
            "WITH ranked AS (
                SELECT
                    id,
                    timestamp,
                    app_name,
                    window_title,
                    screenshot_path,
                    ocr_text,
                    category,
                    duration,
                    COALESCE(RTRIM(browser_url, '/'), '') as browser_url,
                    ROW_NUMBER() OVER (
                        PARTITION BY
                            app_name,
                            CASE
                                WHEN browser_url IS NOT NULL AND browser_url != '' THEN RTRIM(browser_url, '/')
                                ELSE window_title
                            END
                        ORDER BY timestamp DESC, id DESC
                    ) as rn,
                    SUM(duration) OVER (
                        PARTITION BY
                            app_name,
                            CASE
                                WHEN browser_url IS NOT NULL AND browser_url != '' THEN RTRIM(browser_url, '/')
                                ELSE window_title
                            END
                    ) as total_duration
                FROM activities
                WHERE timestamp >= ?1 AND timestamp < ?2
             )
             SELECT
                id,
                timestamp,
                app_name,
                window_title,
                screenshot_path,
                ocr_text,
                category,
                total_duration,
                browser_url
             FROM ranked
             WHERE rn = 1
             ORDER BY timestamp DESC, id DESC
             LIMIT ?3 OFFSET ?4",
        )?;

        let activities: Vec<Activity> = stmt
            .query_map(params![start_ts, end_ts, limit_val, offset_val], |row| {
                let browser_url: String = row.get(8)?;
                Ok(Activity {
                    id: Some(row.get(0)?),
                    timestamp: row.get(1)?,
                    app_name: row.get(2)?,
                    window_title: row.get(3)?,
                    screenshot_path: row.get(4)?,
                    ocr_text: row.get(5)?,
                    category: row.get(6)?,
                    duration: row.get(7)?,
                    browser_url: if browser_url.is_empty() {
                        None
                    } else {
                        Some(browser_url)
                    },
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(activities)
    }

    /// 保存每日报告
    pub fn save_report(&self, report: &DailyReport) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO daily_reports (date, content, ai_mode, model_name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                report.date,
                report.content,
                report.ai_mode,
                report.model_name,
                report.created_at,
            ],
        )?;

        Ok(())
    }

    /// 获取每日报告
    pub fn get_report(&self, date: &str) -> Result<Option<DailyReport>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let result = conn.query_row(
            "SELECT date, content, ai_mode, model_name, created_at FROM daily_reports WHERE date = ?1",
            params![date],
            |row| {
                Ok(DailyReport {
                    date: row.get(0)?,
                    content: row.get(1)?,
                    ai_mode: row.get(2)?,
                    model_name: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        );

        match result {
            Ok(report) => Ok(Some(report)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e)),
        }
    }

    /// 获取指定日期的所有截图路径
    pub fn get_screenshots(&self, date: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let start_ts = {
            let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|e| AppError::Config(e.to_string()))?;
            safe_local_timestamp(date_parsed.and_hms_opt(0, 0, 0).unwrap())
        };
        let end_ts = start_ts + 86400;

        let mut stmt = conn.prepare(
            "SELECT screenshot_path FROM activities WHERE timestamp >= ?1 AND timestamp < ?2 ORDER BY timestamp ASC"
        )?;

        let paths: Vec<String> = stmt
            .query_map(params![start_ts, end_ts], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(paths)
    }

    /// 保存小时摘要
    pub fn save_hourly_summary(&self, summary: &HourlySummary) -> Result<i64> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO hourly_summaries 
             (date, hour, summary, main_apps, activity_count, total_duration, representative_screenshots, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                summary.date,
                summary.hour,
                summary.summary,
                summary.main_apps,
                summary.activity_count,
                summary.total_duration,
                summary.representative_screenshots,
                summary.created_at,
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// 获取指定日期的所有小时摘要
    pub fn get_hourly_summaries(&self, date: &str) -> Result<Vec<HourlySummary>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, date, hour, summary, main_apps, activity_count, total_duration, representative_screenshots, created_at 
             FROM hourly_summaries 
             WHERE date = ?1 
             ORDER BY hour ASC"
        )?;

        let summaries: Vec<HourlySummary> = stmt
            .query_map(params![date], |row| {
                Ok(HourlySummary {
                    id: Some(row.get(0)?),
                    date: row.get(1)?,
                    hour: row.get(2)?,
                    summary: row.get(3)?,
                    main_apps: row.get(4)?,
                    activity_count: row.get(5)?,
                    total_duration: row.get(6)?,
                    representative_screenshots: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(summaries)
    }

    /// 获取指定小时的活动数据（用于生成小时摘要）
    pub fn get_hourly_activities(&self, date: &str, hour: i32) -> Result<Vec<Activity>> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let date_parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| AppError::Config(e.to_string()))?;
        let h = (hour as u32).min(23);
        let start_ts = safe_local_timestamp(date_parsed.and_hms_opt(h, 0, 0).unwrap());
        let end_ts = start_ts + 3600; // 1小时

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, app_name, window_title, screenshot_path, ocr_text, category, duration, browser_url 
             FROM activities 
             WHERE timestamp >= ?1 AND timestamp < ?2 
             ORDER BY timestamp ASC"
        )?;

        let activities: Vec<Activity> = stmt
            .query_map(params![start_ts, end_ts], |row| {
                Ok(Activity {
                    id: Some(row.get(0)?),
                    timestamp: row.get(1)?,
                    app_name: row.get(2)?,
                    window_title: row.get(3)?,
                    screenshot_path: row.get(4)?,
                    ocr_text: row.get(5)?,
                    category: row.get(6)?,
                    duration: row.get(7)?,
                    browser_url: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(activities)
    }

    /// 检查指定小时是否已有摘要
    pub fn has_hourly_summary(&self, date: &str, hour: i32) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM hourly_summaries WHERE date = ?1 AND hour = ?2",
            params![date, hour],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// 获取历史应用列表（按使用时长排序）
    /// 返回去重后的应用名列表
    pub fn get_recent_apps(&self, limit: u32) -> Result<Vec<String>> {
        use std::collections::HashMap;

        let conn = self.conn.lock().map_err(|e| {
            AppError::Database(rusqlite::Error::InvalidParameterName(e.to_string()))
        })?;

        // 查询所有应用并在 Rust 侧做归一化合并，避免 work-review / Work Review 分裂成两条
        let mut stmt = conn.prepare(
            "SELECT app_name, SUM(duration) as total_duration 
             FROM activities 
             GROUP BY app_name 
             ORDER BY total_duration DESC",
        )?;

        let rows: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut merged: HashMap<String, i64> = HashMap::new();
        for (raw_name, duration) in rows {
            let normalized = crate::monitor::normalize_display_app_name(&raw_name);
            *merged.entry(normalized).or_insert(0) += duration;
        }

        let mut apps: Vec<(String, i64)> = merged.into_iter().collect();
        apps.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        Ok(apps
            .into_iter()
            .take(limit as usize)
            .map(|(name, _)| name)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{Activity, Database};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("work-review-{name}-{unique}.db"))
    }

    #[test]
    fn 时间线应使用最新记录详情并累计分组时长() {
        let db_path = temp_db_path("timeline");
        let db = Database::new(&db_path).expect("创建测试数据库失败");
        let now = chrono::Local::now().timestamp();
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();

        let records = vec![
            Activity {
                id: None,
                timestamp: now - 30,
                app_name: "Code".to_string(),
                window_title: "文件A".to_string(),
                screenshot_path: "shot-a.jpg".to_string(),
                ocr_text: Some("old".to_string()),
                category: "development".to_string(),
                duration: 10,
                browser_url: None,
            },
            Activity {
                id: None,
                timestamp: now - 10,
                app_name: "Code".to_string(),
                window_title: "文件A".to_string(),
                screenshot_path: "shot-b.jpg".to_string(),
                ocr_text: Some("new".to_string()),
                category: "development".to_string(),
                duration: 25,
                browser_url: None,
            },
            Activity {
                id: None,
                timestamp: now - 5,
                app_name: "Code".to_string(),
                window_title: "文件B".to_string(),
                screenshot_path: "shot-c.jpg".to_string(),
                ocr_text: None,
                category: "development".to_string(),
                duration: 15,
                browser_url: None,
            },
        ];

        for activity in &records {
            db.insert_activity(activity).expect("插入测试数据失败");
        }

        let timeline = db.get_timeline(&date, None, None).expect("读取时间线失败");
        let file_a = timeline
            .iter()
            .find(|activity| activity.window_title == "文件A")
            .expect("未找到文件A记录");
        let file_b = timeline
            .iter()
            .find(|activity| activity.window_title == "文件B")
            .expect("未找到文件B记录");

        assert_eq!(timeline.len(), 2);
        assert_eq!(file_a.duration, 35);
        assert_eq!(file_a.screenshot_path, "shot-b.jpg");
        assert_eq!(file_a.ocr_text.as_deref(), Some("new"));
        assert_eq!(file_b.duration, 15);

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn 今日统计应合并应用别名避免重复显示() {
        let db_path = temp_db_path("daily-stats-merge");
        let db = Database::new(&db_path).expect("创建测试数据库失败");
        let now = chrono::Local::now().timestamp();
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();

        let records = vec![
            Activity {
                id: None,
                timestamp: now - 60,
                app_name: "work-review".to_string(),
                window_title: "主窗口".to_string(),
                screenshot_path: "wr-a.jpg".to_string(),
                ocr_text: None,
                category: "development".to_string(),
                duration: 540,
                browser_url: None,
            },
            Activity {
                id: None,
                timestamp: now - 30,
                app_name: "Work Review".to_string(),
                window_title: "设置".to_string(),
                screenshot_path: "wr-b.jpg".to_string(),
                ocr_text: None,
                category: "development".to_string(),
                duration: 540,
                browser_url: None,
            },
            Activity {
                id: None,
                timestamp: now - 10,
                app_name: "Code".to_string(),
                window_title: "main.rs".to_string(),
                screenshot_path: "code.jpg".to_string(),
                ocr_text: None,
                category: "development".to_string(),
                duration: 300,
                browser_url: None,
            },
        ];

        for activity in &records {
            db.insert_activity(activity).expect("插入测试数据失败");
        }

        let stats = db
            .get_daily_stats_with_work_time(&date, 9, 18, 0, 0)
            .expect("读取今日统计失败");

        let work_review = stats
            .app_usage
            .iter()
            .find(|item| item.app_name == "Work Review")
            .expect("未找到 Work Review 聚合结果");

        assert_eq!(work_review.duration, 1080);
        assert_eq!(work_review.count, 2);
        assert_eq!(
            stats
                .app_usage
                .iter()
                .filter(|item| item.app_name == "work-review")
                .count(),
            0
        );
        assert_eq!(stats.app_usage.len(), 2);

        let _ = std::fs::remove_file(db_path);
    }
}
