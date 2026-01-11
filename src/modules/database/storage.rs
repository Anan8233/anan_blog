use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

/// 页面类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PageType {
    Index,
    Category,
    Item,
}

/// 页面结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub slug: String, // URL slug (e.g., "grape", "grape-tizi")
    pub page_type: PageType,
    pub title: String,
    pub content: String,          // HTML content
    pub category: Option<String>, // For items/categories
    pub updated_at: String,
}

/// 附件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAttachment {
    pub id: String,
    pub slug: String,          // Associated item slug
    pub filename: String,      // Stored filename
    pub original_name: String, // Original filename
    pub mime_type: String,
    pub file_data: Vec<u8>, // Binary content
    pub file_size: usize,
    pub updated_at: String,
}

/// 站点统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteStats {
    pub total_categories: usize,
    pub total_items: usize,
    pub total_attachments: usize,
    pub last_compiled: Option<String>,
}

/// 统一的存储数据库
pub struct StorageDB {
    conn: Connection,
}

impl StorageDB {
    /// 创建新的存储数据库
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // 创建页面表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pages (
                id TEXT PRIMARY KEY,
                slug TEXT NOT NULL UNIQUE,
                page_type TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建索引以加速查询
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pages_slug ON pages(slug)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pages_type ON pages(page_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pages_category ON pages(category)",
            [],
        )?;

        // 创建附件表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS attachments (
                id TEXT PRIMARY KEY,
                slug TEXT NOT NULL,
                filename TEXT NOT NULL,
                original_name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                file_data BLOB NOT NULL,
                file_size INTEGER NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_attachments_slug ON attachments(slug)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_attachments_filename ON attachments(filename)",
            [],
        )?;

        // 创建站点元数据表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS site_metadata (
                key TEXT PRIMARY KEY UNIQUE,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // 初始化默认元数据
        conn.execute(
            "INSERT OR IGNORE INTO site_metadata (key, value) VALUES (?, ?)",
            ["last_compiled", ""],
        )?;

        Ok(Self { conn })
    }

    // ==================== 页面操作 ====================

    /// 保存页面
    pub fn save_page(&self, page: &Page) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let page_type_str = match page.page_type {
            PageType::Index => "index",
            PageType::Category => "category",
            PageType::Item => "item",
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO pages (id, slug, page_type, title, content, category, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![page.id, page.slug, page_type_str, page.title, page.content, page.category, now],
        )?;

        Ok(())
    }

    /// 获取页面
    pub fn get_page(&self, slug: &str) -> Result<Option<Page>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, page_type, title, content, category, updated_at
             FROM pages WHERE slug = ?",
        )?;

        let mut rows = stmt.query(params![slug])?;

        if let Some(row) = rows.next()? {
            let page_type_str: String = row.get(2)?;
            let page_type = match page_type_str.as_str() {
                "index" => PageType::Index,
                "category" => PageType::Category,
                "item" => PageType::Item,
                _ => PageType::Item,
            };

            let page = Page {
                id: row.get(0)?,
                slug: row.get(1)?,
                page_type,
                title: row.get(3)?,
                content: row.get(4)?,
                category: row.get(5)?,
                updated_at: row.get(6)?,
            };
            Ok(Some(page))
        } else {
            Ok(None)
        }
    }

    /// 获取所有页面
    pub fn get_all_pages(&self) -> Result<Vec<Page>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, page_type, title, content, category, updated_at
             FROM pages ORDER BY slug",
        )?;

        let mut rows = stmt.query([])?;
        let mut pages = Vec::new();

        while let Some(row) = rows.next()? {
            let page_type_str: String = row.get(2)?;
            let page_type = match page_type_str.as_str() {
                "index" => PageType::Index,
                "category" => PageType::Category,
                "item" => PageType::Item,
                _ => PageType::Item,
            };

            pages.push(Page {
                id: row.get(0)?,
                slug: row.get(1)?,
                page_type,
                title: row.get(3)?,
                content: row.get(4)?,
                category: row.get(5)?,
                updated_at: row.get(6)?,
            });
        }
        Ok(pages)
    }

    /// 按类型获取页面
    pub fn get_pages_by_type(&self, page_type: PageType) -> Result<Vec<Page>> {
        let page_type_str = match &page_type {
            PageType::Index => "index",
            PageType::Category => "category",
            PageType::Item => "item",
        };

        let mut stmt = self.conn.prepare(
            "SELECT id, slug, page_type, title, content, category, updated_at
             FROM pages WHERE page_type = ? ORDER BY slug",
        )?;

        let mut rows = stmt.query(params![page_type_str])?;
        let mut pages = Vec::new();
        let page_type = page_type; // 重新绑定

        while let Some(row) = rows.next()? {
            pages.push(Page {
                id: row.get(0)?,
                slug: row.get(1)?,
                page_type: page_type.clone(),
                title: row.get(3)?,
                content: row.get(4)?,
                category: row.get(5)?,
                updated_at: row.get(6)?,
            });
        }
        Ok(pages)
    }

    /// 获取某分类的所有项目页面
    pub fn get_items_by_category(&self, category: &str) -> Result<Vec<Page>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, page_type, title, content, category, updated_at
             FROM pages WHERE page_type = 'item' AND category = ? ORDER BY slug",
        )?;

        let mut rows = stmt.query(params![category])?;
        let mut pages = Vec::new();

        while let Some(row) = rows.next()? {
            pages.push(Page {
                id: row.get(0)?,
                slug: row.get(1)?,
                page_type: PageType::Item,
                title: row.get(3)?,
                content: row.get(4)?,
                category: row.get(5)?,
                updated_at: row.get(6)?,
            });
        }
        Ok(pages)
    }

    /// 删除页面
    pub fn delete_page(&self, slug: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM pages WHERE slug = ?", params![slug])?;
        Ok(rows > 0)
    }

    /// 清除所有页面
    pub fn clear_pages(&self) -> Result<()> {
        self.conn.execute("DELETE FROM pages", [])?;
        Ok(())
    }

    // ==================== 附件操作 ====================

    /// 保存附件
    pub fn save_attachment(&self, attachment: &StoredAttachment) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO attachments
             (id, slug, filename, original_name, mime_type, file_data, file_size, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                attachment.id,
                attachment.slug,
                attachment.filename,
                attachment.original_name,
                attachment.mime_type,
                attachment.file_data,
                attachment.file_size,
                now
            ],
        )?;

        Ok(())
    }

    /// 获取附件
    pub fn get_attachment(&self, filename: &str) -> Result<Option<StoredAttachment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, filename, original_name, mime_type, file_data, file_size, updated_at
             FROM attachments WHERE filename = ?",
        )?;

        let mut rows = stmt.query(params![filename])?;

        if let Some(row) = rows.next()? {
            let attachment = StoredAttachment {
                id: row.get(0)?,
                slug: row.get(1)?,
                filename: row.get(2)?,
                original_name: row.get(3)?,
                mime_type: row.get(4)?,
                file_data: row.get(5)?,
                file_size: row.get(6)?,
                updated_at: row.get(7)?,
            };
            Ok(Some(attachment))
        } else {
            Ok(None)
        }
    }

    /// 获取某项目的所有附件
    pub fn get_attachments_by_slug(&self, slug: &str) -> Result<Vec<StoredAttachment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, filename, original_name, mime_type, file_data, file_size, updated_at
             FROM attachments WHERE slug = ? ORDER BY filename",
        )?;

        let mut rows = stmt.query(params![slug])?;
        let mut attachments = Vec::new();

        while let Some(row) = rows.next()? {
            attachments.push(StoredAttachment {
                id: row.get(0)?,
                slug: row.get(1)?,
                filename: row.get(2)?,
                original_name: row.get(3)?,
                mime_type: row.get(4)?,
                file_data: row.get(5)?,
                file_size: row.get(6)?,
                updated_at: row.get(7)?,
            });
        }
        Ok(attachments)
    }

    /// 获取所有附件
    pub fn get_all_attachments(&self) -> Result<Vec<StoredAttachment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, filename, original_name, mime_type, file_data, file_size, updated_at
             FROM attachments ORDER BY slug, filename",
        )?;

        let mut rows = stmt.query([])?;
        let mut attachments = Vec::new();

        while let Some(row) = rows.next()? {
            attachments.push(StoredAttachment {
                id: row.get(0)?,
                slug: row.get(1)?,
                filename: row.get(2)?,
                original_name: row.get(3)?,
                mime_type: row.get(4)?,
                file_data: row.get(5)?,
                file_size: row.get(6)?,
                updated_at: row.get(7)?,
            });
        }
        Ok(attachments)
    }

    /// 删除附件
    pub fn delete_attachment(&self, filename: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM attachments WHERE filename = ?",
            params![filename],
        )?;
        Ok(rows > 0)
    }

    /// 删除某项目的所有附件
    pub fn delete_attachments_by_slug(&self, slug: &str) -> Result<usize> {
        let rows = self
            .conn
            .execute("DELETE FROM attachments WHERE slug = ?", params![slug])?;
        Ok(rows)
    }

    /// 清除所有附件
    pub fn clear_attachments(&self) -> Result<()> {
        self.conn.execute("DELETE FROM attachments", [])?;
        Ok(())
    }

    // ==================== 元数据操作 ====================

    /// 设置元数据
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO site_metadata (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    /// 获取元数据
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM site_metadata WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// 更新编译时间
    pub fn update_compile_time(&self) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.set_metadata("last_compiled", &now)
    }

    /// 获取最后编译时间
    pub fn get_last_compiled(&self) -> Result<Option<String>> {
        self.get_metadata("last_compiled")
    }

    // ==================== 搜索功能 ====================

    /// 搜索页面
    pub fn search_pages(&self, query: &str, limit: usize) -> Result<Vec<Page>> {
        let search_pattern = format!("%{}%", query);

        let mut stmt = self.conn.prepare(
            "SELECT id, slug, page_type, title, content, category, updated_at
             FROM pages
             WHERE (title LIKE ? OR content LIKE ?)
             AND page_type != 'index'
             ORDER BY
                CASE
                    WHEN title LIKE ? THEN 2
                    WHEN title LIKE ? THEN 1
                    ELSE 0
                END DESC,
                updated_at DESC
             LIMIT ?"
        )?;

        let mut rows = stmt.query(params![
            search_pattern,
            search_pattern,
            search_pattern,
            format!("%{}%", query),
            limit
        ])?;

        let mut pages = Vec::new();

        while let Some(row) = rows.next()? {
            let page_type_str: String = row.get(2)?;
            let page_type = match page_type_str.as_str() {
                "index" => PageType::Index,
                "category" => PageType::Category,
                "item" => PageType::Item,
                _ => PageType::Item,
            };

            pages.push(Page {
                id: row.get(0)?,
                slug: row.get(1)?,
                page_type,
                title: row.get(3)?,
                content: row.get(4)?,
                category: row.get(5)?,
                updated_at: row.get(6)?,
            });
        }

        Ok(pages)
    }

    /// 获取搜索结果数量
    pub fn search_pages_count(&self, query: &str) -> Result<usize> {
        let search_pattern = format!("%{}%", query);

        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM pages
             WHERE (title LIKE ? OR content LIKE ?)
             AND page_type != 'index'",
            params![search_pattern, search_pattern],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    // ==================== 统计信息 ====================

    /// 获取站点统计信息
    pub fn get_stats(&self) -> Result<SiteStats> {
        let total_categories: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pages WHERE page_type = 'category'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_items: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pages WHERE page_type = 'item'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_attachments: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM attachments", [], |row| row.get(0))
            .unwrap_or(0);

        let last_compiled = self.get_last_compiled()?;

        Ok(SiteStats {
            total_categories: total_categories as usize,
            total_items: total_items as usize,
            total_attachments: total_attachments as usize,
            last_compiled,
        })
    }

    // ==================== 批量操作 ====================

    /// 清空所有存储的数据
    pub fn clear_all(&mut self) -> Result<()> {
        self.clear_pages()?;
        self.clear_attachments()?;
        self.conn.execute("DELETE FROM site_metadata", [])?;
        self.conn.execute(
            "INSERT OR IGNORE INTO site_metadata (key, value) VALUES (?, ?)",
            ["last_compiled", ""],
        )?;
        Ok(())
    }

    /// 执行批量保存（事务）
    pub fn save_pages_batch(&mut self, pages: &[Page]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO pages (id, slug, page_type, title, content, category, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )?;
            let now = chrono::Utc::now().to_rfc3339();

            for page in pages {
                let page_type_str = match page.page_type {
                    PageType::Index => "index",
                    PageType::Category => "category",
                    PageType::Item => "item",
                };
                stmt.execute(params![
                    page.id,
                    page.slug,
                    page_type_str,
                    page.title,
                    page.content,
                    page.category,
                    now
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// 批量保存附件（事务）
    pub fn save_attachments_batch(&mut self, attachments: &[StoredAttachment]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO attachments
                 (id, slug, filename, original_name, mime_type, file_data, file_size, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )?;
            let now = chrono::Utc::now().to_rfc3339();

            for attachment in attachments {
                stmt.execute(params![
                    attachment.id,
                    attachment.slug,
                    attachment.filename,
                    attachment.original_name,
                    attachment.mime_type,
                    attachment.file_data,
                    attachment.file_size,
                    now
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

/// 根据文件名获取 MIME 类型
pub fn get_mime_type(filename: &str) -> &'static str {
    if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else if filename.ends_with(".png") {
        "image/png"
    } else if filename.ends_with(".gif") {
        "image/gif"
    } else if filename.ends_with(".svg") {
        "image/svg+xml"
    } else if filename.ends_with(".webp") {
        "image/webp"
    } else if filename.ends_with(".pdf") {
        "application/pdf"
    } else if filename.ends_with(".zip") {
        "application/zip"
    } else if filename.ends_with(".mp4") {
        "video/mp4"
    } else if filename.ends_with(".mp3") {
        "audio/mpeg"
    } else if filename.ends_with(".txt") {
        "text/plain"
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_creation() {
        let temp_path = "/tmp/test_storage.db";
        let _ = std::fs::remove_file(temp_path);

        let db = StorageDB::new(temp_path);
        assert!(db.is_ok());

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_save_and_get_page() {
        let temp_path = "/tmp/test_storage_page.db";
        let _ = std::fs::remove_file(temp_path);

        let db = StorageDB::new(temp_path).unwrap();

        let page = Page {
            id: "test-1".to_string(),
            slug: "test-page".to_string(),
            page_type: PageType::Item,
            title: "Test Page".to_string(),
            content: "<h1>Hello</h1>".to_string(),
            category: Some("test".to_string()),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        assert!(db.save_page(&page).is_ok());
        assert!(db.get_page("test-page").unwrap().is_some());

        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_save_and_get_attachment() {
        let temp_path = "/tmp/test_storage_attachment.db";
        let _ = std::fs::remove_file(temp_path);

        let db = StorageDB::new(temp_path).unwrap();

        let attachment = StoredAttachment {
            id: "att-1".to_string(),
            slug: "test-item".to_string(),
            filename: "test_image.jpg".to_string(),
            original_name: "my_photo.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
            file_data: vec![0x89, 0x50, 0x4E, 0x47], // PNG header
            file_size: 4,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        assert!(db.save_attachment(&attachment).is_ok());
        assert!(db.get_attachment("test_image.jpg").unwrap().is_some());

        std::fs::remove_file(temp_path).ok();
    }
}
