use chrono::Utc;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 评论数据结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Comment {
    pub id: String,
    pub slug: String,
    pub author: String,
    pub content: String,
    pub created_at: String,
    pub website: Option<String>,
}

/// 创建评论请求
#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub slug: String,
    pub author: String,
    pub content: String,
    pub website: Option<String>,
}

/// 评论数据库管理器
pub struct CommentDB {
    conn: Connection,
}

impl CommentDB {
    /// 创建新的评论数据库
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // 创建评论表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS comments (
                id TEXT PRIMARY KEY,
                slug TEXT NOT NULL,
                author TEXT NOT NULL,
                content TEXT NOT NULL,
                website TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // 创建索引以加速查询
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_comments_slug ON comments(slug)",
            [],
        )?;

        Ok(Self { conn })
    }

    /// 添加新评论
    pub fn add_comment(&self, request: CreateCommentRequest) -> Result<Comment> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO comments (id, slug, author, content, website, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                id,
                request.slug,
                request.author,
                request.content,
                request.website,
                now
            ],
        )?;

        Ok(Comment {
            id,
            slug: request.slug,
            author: request.author,
            content: request.content,
            created_at: now,
            website: request.website,
        })
    }

    /// 获取某篇文章的所有评论
    pub fn get_comments_by_slug(&self, slug: &str) -> Result<Vec<Comment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, author, content, website, created_at
             FROM comments WHERE slug = ? ORDER BY created_at DESC",
        )?;

        let comments = stmt
            .query_map(params![slug], |row| {
                Ok(Comment {
                    id: row.get(0)?,
                    slug: row.get(1)?,
                    author: row.get(2)?,
                    content: row.get(3)?,
                    website: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(comments)
    }

    /// 获取所有评论（按文章分组）
    pub fn get_all_comments(&self) -> Result<Vec<Comment>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, slug, author, content, website, created_at
             FROM comments ORDER BY created_at DESC",
        )?;

        let comments = stmt
            .query_map([], |row| {
                Ok(Comment {
                    id: row.get(0)?,
                    slug: row.get(1)?,
                    author: row.get(2)?,
                    content: row.get(3)?,
                    website: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(comments)
    }

    /// 删除评论
    pub fn delete_comment(&self, id: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM comments WHERE id = ?", params![id])?;
        Ok(rows > 0)
    }

    /// 获取评论统计
    pub fn get_comment_stats(&self) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT slug, COUNT(*) as count FROM comments GROUP BY slug ORDER BY count DESC",
        )?;

        let stats = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(stats)
    }
}

/// 简化的评论统计（用于模板渲染）
#[derive(Debug, Serialize)]
pub struct CommentCount {
    pub slug: String,
    pub count: i64,
}
