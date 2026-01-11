use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::config::Config;

/// 访问来源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VisitSource {
    Direct,          // 直接访问
    SearchEngine,    // 搜索引擎
    SocialMedia,     // 社交媒体
    ExternalLink,    // 外部链接
    Unknown,         // 未知
}

impl Default for VisitSource {
    fn default() -> Self {
        VisitSource::Unknown
    }
}

impl std::str::FromStr for VisitSource {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "direct" => Ok(VisitSource::Direct),
            "search" => Ok(VisitSource::SearchEngine),
            "social" => Ok(VisitSource::SocialMedia),
            "external" => Ok(VisitSource::ExternalLink),
            _ => Ok(VisitSource::Unknown),
        }
    }
}

impl ToString for VisitSource {
    fn to_string(&self) -> String {
        match self {
            VisitSource::Direct => "direct".to_string(),
            VisitSource::SearchEngine => "search".to_string(),
            VisitSource::SocialMedia => "social".to_string(),
            VisitSource::ExternalLink => "external".to_string(),
            VisitSource::Unknown => "unknown".to_string(),
        }
    }
}

/// 访问记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisitRecord {
    pub id: String,
    pub page_slug: String,      // 访问的页面 slug
    pub ip_address: String,     // IP 地址
    pub user_agent: String,     // 用户代理
    pub referer: String,        // 来源页面
    pub source: VisitSource,    // 来源类型
    pub search_engine: Option<String>, // 搜索引擎名称（如有）
    pub search_keyword: Option<String>, // 搜索关键词（如有）
    pub visit_time: String,     // 访问时间
    pub country: Option<String>, // 国家（可选）
    pub city: Option<String>,    // 城市（可选）
}

/// 每日统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub total_visits: i64,
    pub unique_visitors: i64,
    pub page_views: i64,
}

/// 来源统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStats {
    pub source: String,
    pub count: i64,
    pub percentage: f64,
}

/// 热门页面
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularPage {
    pub slug: String,
    pub title: String,
    pub visits: i64,
}

/// 访问统计数据库
pub struct AnalyticsDB {
    conn: Connection,
}

impl AnalyticsDB {
    /// 创建新的访问统计数据库
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // 创建访问记录表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS visits (
                id TEXT PRIMARY KEY,
                page_slug TEXT NOT NULL,
                ip_address TEXT NOT NULL,
                user_agent TEXT NOT NULL,
                referer TEXT NOT NULL,
                source TEXT NOT NULL,
                search_engine TEXT,
                search_keyword TEXT,
                visit_time TEXT NOT NULL,
                country TEXT,
                city TEXT
            )",
            [],
        )?;

        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_visits_page ON visits(page_slug)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_visits_time ON visits(visit_time)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_visits_ip ON visits(ip_address)",
            [],
        )?;

        // 创建每日统计视图
        conn.execute(
            "CREATE VIEW IF NOT EXISTS daily_stats AS
             SELECT
                date(visit_time) as date,
                COUNT(*) as total_visits,
                COUNT(DISTINCT ip_address) as unique_visitors,
                COUNT(*) as page_views
             FROM visits
             GROUP BY date(visit_time)",
            [],
        )?;

        Ok(Self { conn })
    }

    /// 记录一次访问
    pub fn record_visit(&self, record: &VisitRecord) -> Result<()> {
        self.conn.execute(
            "INSERT INTO visits
             (id, page_slug, ip_address, user_agent, referer, source, search_engine, search_keyword, visit_time, country, city)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                record.id,
                record.page_slug,
                record.ip_address,
                record.user_agent,
                record.referer,
                record.source.to_string(),
                record.search_engine,
                record.search_keyword,
                record.visit_time,
                record.country,
                record.city
            ],
        )?;
        Ok(())
    }

    /// 获取每日统计
    pub fn get_daily_stats(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<DailyStats>> {
        let query = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                format!(
                    "SELECT date, total_visits, unique_visitors, page_views FROM daily_stats
                     WHERE date BETWEEN '{}' AND '{}' ORDER BY date DESC",
                    start, end
                )
            }
            (Some(start), None) => {
                format!(
                    "SELECT date, total_visits, unique_visitors, page_views FROM daily_stats
                     WHERE date >= '{}' ORDER BY date DESC",
                    start
                )
            }
            (None, Some(end)) => {
                format!(
                    "SELECT date, total_visits, unique_visitors, page_views FROM daily_stats
                     WHERE date <= '{}' ORDER BY date DESC",
                    end
                )
            }
            _ => {
                "SELECT date, total_visits, unique_visitors, page_views FROM daily_stats
                 ORDER BY date DESC LIMIT 30".to_string()
            }
        };

        let mut stmt = self.conn.prepare(&query)?;
        let mut rows = stmt.query([])?;
        let mut stats = Vec::new();

        while let Some(row) = rows.next()? {
            stats.push(DailyStats {
                date: row.get(0)?,
                total_visits: row.get(1)?,
                unique_visitors: row.get(2)?,
                page_views: row.get(3)?,
            });
        }
        Ok(stats)
    }

    /// 获取来源统计
    pub fn get_source_stats(&self, limit: Option<usize>) -> Result<Vec<SourceStats>> {
        let limit = limit.unwrap_or(10);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT source, COUNT(*) as count FROM visits GROUP BY source ORDER BY count DESC LIMIT {}",
            limit
        ))?;

        let mut rows = stmt.query([])?;
        let mut stats = Vec::new();
        let total: i64 = {
            let mut total_stmt = self.conn.prepare("SELECT COUNT(*) FROM visits")?;
            total_stmt.query_row([], |row| row.get(0))?
        };

        while let Some(row) = rows.next()? {
            let count: i64 = row.get(1)?;
            stats.push(SourceStats {
                source: row.get(0)?,
                count,
                percentage: if total > 0 { (count as f64 / total as f64) * 100.0 } else { 0.0 },
            });
        }
        Ok(stats)
    }

    /// 获取热门页面
    pub fn get_popular_pages(&self, limit: Option<usize>) -> Result<Vec<PopularPage>> {
        let limit = limit.unwrap_or(10);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT page_slug, COUNT(*) as visits FROM visits GROUP BY page_slug ORDER BY visits DESC LIMIT {}",
            limit
        ))?;

        let mut rows = stmt.query([])?;
        let mut pages = Vec::new();

        while let Some(row) = rows.next()? {
            pages.push(PopularPage {
                slug: row.get(0)?,
                title: row.get(0)?, // 可以后续从 storage 获取真实标题
                visits: row.get(1)?,
            });
        }
        Ok(pages)
    }

    /// 获取总访问量
    pub fn get_total_visits(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM visits", [], |row| row.get(0))
    }

    /// 获取独立访客数
    pub fn get_unique_visitors(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(DISTINCT ip_address) FROM visits", [], |row| row.get(0))
    }

    /// 获取搜索引擎关键词统计
    pub fn get_search_keywords(&self, limit: Option<usize>) -> Result<Vec<(String, i64)>> {
        let limit = limit.unwrap_or(20);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT search_keyword, COUNT(*) as count FROM visits
             WHERE search_keyword IS NOT NULL AND search_keyword != ''
             GROUP BY search_keyword ORDER BY count DESC LIMIT {}",
            limit
        ))?;

        let mut rows = stmt.query([])?;
        let mut keywords = Vec::new();

        while let Some(row) = rows.next()? {
            keywords.push((row.get(0)?, row.get(1)?));
        }
        Ok(keywords)
    }

    /// 获取特定页面的访问量
    pub fn get_page_visits(&self, page_slug: &str) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM visits WHERE page_slug = ?",
            params![page_slug],
            |row| row.get(0),
        )
    }

    /// 清理旧记录（保留最近 N 天）
    pub fn cleanup_old_records(&self, days_to_keep: i64) -> Result<usize> {
        let cutoff_date = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days_to_keep))
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();

        let rows = self.conn.execute(
            "DELETE FROM visits WHERE date(visit_time) < ?",
            params![cutoff_date],
        )?;
        Ok(rows)
    }
}

/// 解析用户代理，提取浏览器和操作系统信息
pub fn parse_user_agent(ua: &str) -> (String, String) {
    let ua = ua.to_lowercase();

    // 检测浏览器
    let browser = if ua.contains("chrome") && !ua.contains("chromium") {
        "Chrome"
    } else if ua.contains("firefox") {
        "Firefox"
    } else if ua.contains("safari") && !ua.contains("chrome") {
        "Safari"
    } else if ua.contains("edge") {
        "Edge"
    } else if ua.contains("opera") || ua.contains("opr") {
        "Opera"
    } else {
        "Other"
    }
    .to_string();

    // 检测操作系统
    let os = if ua.contains("windows") {
        "Windows"
    } else if ua.contains("mac os") || ua.contains("macos") {
        "macOS"
    } else if ua.contains("iphone") || ua.contains("ipad") {
        "iOS"
    } else if ua.contains("android") {
        "Android"
    } else if ua.contains("linux") {
        "Linux"
    } else if ua.contains("chromeos") {
        "ChromeOS"
    } else {
        "Other"
    }
    .to_string();

    (browser, os)
}

/// 检测访问来源
pub fn detect_source(referer: &str, user_agent: &str) -> (VisitSource, Option<String>, Option<String>) {
    let referer_lower = referer.to_lowercase();

    // 直接访问
    if referer.is_empty() || referer == "direct" {
        return (VisitSource::Direct, None, None);
    }

    // 搜索引擎检测
    let search_engines = [
        ("google", "Google", vec!["q=", "search"]),
        ("bing", "Bing", vec!["q="]),
        ("baidu", "Baidu", vec!["wd=", "word=", "kw="]),
        ("yahoo", "Yahoo", vec!["p=", "q="]),
        ("yandex", "Yandex", vec!["text=", "query="]),
        ("duckduckgo", "DuckDuckGo", vec!["q="]),
    ];

    for (engine_id, engine_name, params) in &search_engines {
        for param in params {
            if referer.contains(param) {
                // 尝试提取关键词
                let keyword = extract_query_param(&referer, param);
                return (
                    VisitSource::SearchEngine,
                    Some(engine_name.to_string()),
                    keyword,
                );
            }
        }
        // 额外检查搜索引擎域名
        if referer.contains(engine_id) {
            return (VisitSource::SearchEngine, Some(engine_name.to_string()), None);
        }
    }

    // 社交媒体检测
    let social_media = [
        "twitter.com",
        "facebook.com",
        "instagram.com",
        "weibo.com",
        "zhihu.com",
        "douban.com",
        "reddit.com",
        "pinterest.com",
        "linkedin.com",
        "tiktok.com",
    ];

    for social in &social_media {
        if referer.contains(social) {
            return (VisitSource::SocialMedia, None, None);
        }
    }

    // 外部链接
    if !referer.is_empty() {
        return (VisitSource::ExternalLink, None, None);
    }

    (VisitSource::Unknown, None, None)
}

/// 从 URL 中提取查询参数值
fn extract_query_param(url: &str, param_name: &str) -> Option<String> {
    if let Some(pos) = url.find(param_name) {
        let start = pos + param_name.len();
        if start < url.len() && &url[start..start + 1] == "=" {
            let end = url[start + 1..]
                .find('&')
                .map(|p| start + 1 + p)
                .unwrap_or(url.len());
            let value = &url[start + 1..end];
            // URL 解码
            return Some(urlencoding::decode(value).map(|cow| cow.into_owned()).unwrap_or_else(|_| value.to_string()));
        }
    }
    None
}

/// 从 IP 地址获取位置信息（简化版本，实际可以使用 GeoIP 数据库）
pub fn get_location_from_ip(ip: &str) -> (Option<String>, Option<String>) {
    // 简化实现：返回 None
    // 实际使用中，可以使用 MaxMind GeoIP 或其他服务
    (None, None)
}

// ==================== API Handler Functions ====================

use actix_web::{web, HttpRequest, HttpResponse, Responder};

/// 获取每日统计
#[derive(Deserialize)]
pub struct DailyStatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_daily_stats(
    query: web::Query<DailyStatsQuery>,
    config: web::Data<Config>,
) -> impl Responder {
    let db_path = config.paths.database_path.to_string_lossy().to_string();

    match AnalyticsDB::new(&db_path) {
        Ok(db) => {
            let stats = db.get_daily_stats(
                query.start_date.as_deref(),
                query.end_date.as_deref(),
            );
            match stats {
                Ok(stats) => HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "stats": stats,
                    "count": stats.len()
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 获取来源统计
#[derive(Deserialize)]
pub struct SourceStatsQuery {
    pub limit: Option<usize>,
}

pub async fn get_source_stats(
    query: web::Query<SourceStatsQuery>,
    config: web::Data<Config>,
) -> impl Responder {
    let db_path = config.paths.database_path.to_string_lossy().to_string();

    match AnalyticsDB::new(&db_path) {
        Ok(db) => {
            let stats = db.get_source_stats(query.limit);
            match stats {
                Ok(stats) => HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "sources": stats,
                    "count": stats.len()
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 获取热门页面
#[derive(Deserialize)]
pub struct PopularPagesQuery {
    pub limit: Option<usize>,
}

pub async fn get_popular_pages(
    query: web::Query<PopularPagesQuery>,
    config: web::Data<Config>,
) -> impl Responder {
    let db_path = config.paths.database_path.to_string_lossy().to_string();

    match AnalyticsDB::new(&db_path) {
        Ok(db) => {
            let pages = db.get_popular_pages(query.limit);
            match pages {
                Ok(pages) => HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "pages": pages,
                    "count": pages.len()
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 获取总统计
pub async fn get_total_stats(config: web::Data<Config>) -> impl Responder {
    let db_path = config.paths.database_path.to_string_lossy().to_string();

    match AnalyticsDB::new(&db_path) {
        Ok(db) => {
            let total_visits = db.get_total_visits().unwrap_or(0);
            let unique_visitors = db.get_unique_visitors().unwrap_or(0);
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "stats": {
                    "total_visits": total_visits,
                    "unique_visitors": unique_visitors
                }
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 获取搜索关键词
#[derive(Deserialize)]
pub struct KeywordsQuery {
    pub limit: Option<usize>,
}

pub async fn get_search_keywords(
    query: web::Query<KeywordsQuery>,
    config: web::Data<Config>,
) -> impl Responder {
    let db_path = config.paths.database_path.to_string_lossy().to_string();

    match AnalyticsDB::new(&db_path) {
        Ok(db) => {
            let keywords = db.get_search_keywords(query.limit);
            match keywords {
                Ok(keywords) => HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "keywords": keywords,
                    "count": keywords.len()
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": e.to_string()
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 记录页面访问（中间件会调用）
pub async fn record_visit(
    req: HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> impl Responder {
    let page_slug = path.into_inner();

    // 获取请求信息
    let ip_address = get_client_ip(&req);
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    let referer = req
        .headers()
        .get("referer")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    // 检测来源
    let (source, search_engine, search_keyword) = detect_source(&referer, &user_agent);

    // 获取位置信息
    let (country, city) = get_location_from_ip(&ip_address);

    // 创建访问记录
    let record = VisitRecord {
        id: uuid::Uuid::new_v4().to_string(),
        page_slug,
        ip_address,
        user_agent,
        referer,
        source,
        search_engine,
        search_keyword,
        visit_time: chrono::Utc::now().to_rfc3339(),
        country,
        city,
    };

    // 保存到数据库
    let db_path = config.paths.database_path.to_string_lossy().to_string();

    match AnalyticsDB::new(&db_path) {
        Ok(db) => {
            db.record_visit(&record).ok();
        }
        Err(_) => {
            // 静默失败，不影响正常请求
        }
    }

    HttpResponse::Ok().finish()
}

/// 从请求中获取客户端 IP
fn get_client_ip(req: &HttpRequest) -> String {
    // 检查 X-Forwarded-For 头（代理场景）
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // X-Forwarded-For 可能包含多个 IP，取第一个
            if let Some(ip) = forwarded_str.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // 检查 X-Real-IP 头
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.to_string();
        }
    }

    // 获取连接远程地址
    if let Some(peer) = req.peer_addr() {
        return peer.ip().to_string();
    }

    "0.0.0.0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_agent() {
        let (browser, os) = parse_user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        );
        assert_eq!(browser, "Chrome");
        assert_eq!(os, "macOS");

        let (browser, os) = parse_user_agent(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
        );
        assert_eq!(browser, "Safari");
        assert_eq!(os, "iOS");
    }

    #[test]
    fn test_detect_source() {
        // 直接访问
        let (source, engine, keyword) = detect_source("", "");
        assert_eq!(source, VisitSource::Direct);

        // Google 搜索
        let (source, engine, keyword) = detect_source(
            "https://www.google.com/search?q=rust+programming",
            "",
        );
        assert_eq!(source, VisitSource::SearchEngine);
        assert_eq!(engine, Some("Google".to_string()));
        assert_eq!(keyword, Some("rust programming".to_string()));

        // 百度搜索
        let (source, engine, keyword) = detect_source(
            "https://www.baidu.com/s?wd=你好世界",
            "",
        );
        assert_eq!(source, VisitSource::SearchEngine);
        assert_eq!(engine, Some("Baidu".to_string()));
        assert_eq!(keyword, Some("你好世界".to_string()));

        // 社交媒体
        let (source, engine, keyword) = detect_source(
            "https://twitter.com/user/status/123",
            "",
        );
        assert_eq!(source, VisitSource::SocialMedia);

        // 外部链接
        let (source, engine, keyword) = detect_source(
            "https://example.com/blog/post",
            "",
        );
        assert_eq!(source, VisitSource::ExternalLink);
    }
}