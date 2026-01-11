use crate::config::Config;
use crate::scanner::{Category, ContentItem, Scanner, SiteContent};
use crate::storage::{Page, PageType, StorageDB};
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

/// 管理员登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

/// 管理员登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

/// 创建分类请求
#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
}

/// 创建文章请求
#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub category: String,
    pub item_name: String,
    pub title: String,
    pub content: String,
    pub date: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_draft: Option<bool>,
}

/// 更新文章请求
#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub category: String,
    pub item_name: String,
    pub title: String,
    pub content: String,
    pub date: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_draft: Option<bool>,
}

/// 导入请求
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub import_type: String, // "full" or "incremental"
    pub target_category: Option<String>, // For incremental import
}

/// 导出响应
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub success: bool,
    pub message: String,
    pub file_count: usize,
    pub data_size: usize,
}

/// 编译器状态
pub struct CompilerState {
    pub is_compiling: bool,
    pub last_compile_result: Option<String>,
}

impl CompilerState {
    pub fn new() -> Self {
        Self {
            is_compiling: false,
            last_compile_result: None,
        }
    }
}

/// 验证管理员密码
fn verify_admin_password(config: &Config, password: &str) -> bool {
    password == config.server.admin_password
}

/// 生成简单的会话令牌
fn generate_token() -> String {
    let uuid = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();
    format!("{}_{}", uuid, timestamp)
}

/// 管理员登录
pub async fn admin_login(
    body: web::Json<LoginRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    let request = body.into_inner();

    if verify_admin_password(&config, &request.password) {
        let token = generate_token();
        HttpResponse::Ok().json(LoginResponse {
            success: true,
            message: "登录成功".to_string(),
            token: Some(token),
        })
    } else {
        HttpResponse::Unauthorized().json(LoginResponse {
            success: false,
            message: "密码错误".to_string(),
            token: None,
        })
    }
}

/// 验证管理员令牌
fn verify_admin_token(config: &Config, token: &str) -> bool {
    // 简单验证：令牌格式正确且未过期（24小时内）
    if !token.contains('_') {
        return false;
    }
    let parts: Vec<&str> = token.split('_').collect();
    if parts.len() != 2 {
        return false;
    }
    // 验证时间戳（24小时有效期）
    if let Ok(timestamp) = parts[1].parse::<i64>() {
        let now = chrono::Utc::now().timestamp();
        let diff = now - timestamp;
        return diff < 86400 && diff > -86400; // 24小时窗口
    }
    false
}

/// 获取站点概览
pub async fn get_admin_overview(req: actix_web::HttpRequest, config: web::Data<Config>) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    match StorageDB::new(&storage_path) {
        Ok(storage) => {
            match storage.get_stats() {
                Ok(stats) => {
                    let last_compiled = storage.get_last_compiled().ok().flatten();

                    // 获取评论统计
                    let comments_path = config.paths.storage_database_path.to_string_lossy().to_string();
                    let comment_stats = crate::comments::CommentDB::new(&comments_path)
                        .and_then(|db| db.get_comment_stats());

                    HttpResponse::Ok().json(serde_json::json!({
                        "status": "success",
                        "overview": {
                            "total_categories": stats.total_categories,
                            "total_items": stats.total_items,
                            "total_attachments": stats.total_attachments,
                            "last_compiled": last_compiled,
                            "total_comments": match comment_stats {
                                Ok(s) => s.iter().map(|(_, count)| count).sum::<i64>(),
                                Err(_) => 0,
                            }
                        }
                    }))
                }
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": format!("获取统计失败: {}", e)
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 获取所有分类
pub async fn get_categories(req: actix_web::HttpRequest, config: web::Data<Config>) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    match StorageDB::new(&storage_path) {
        Ok(storage) => {
            let pages = storage.get_pages_by_type(PageType::Category);
            match pages {
                Ok(category_pages) => {
                    let categories: Vec<serde_json::Value> = category_pages
                        .iter()
                        .map(|page| {
                            // 获取该分类下的文章数量
                            let items = storage.get_items_by_category(&page.slug).unwrap_or_default();
                            serde_json::json!({
                                "slug": page.slug.clone(),
                                "title": page.title.clone(),
                                "description": page.content.clone(), // Description is in content for categories
                                "item_count": items.len()
                            })
                        })
                        .collect();

                    HttpResponse::Ok().json(serde_json::json!({
                        "status": "success",
                        "categories": categories
                    }))
                }
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

/// 获取所有文章（包括草稿）
pub async fn get_items(req: actix_web::HttpRequest, config: web::Data<Config>, query: web::Query<Option<bool>>) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let show_drafts = query.into_inner().unwrap_or(false);
    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    match StorageDB::new(&storage_path) {
        Ok(storage) => {
            let all_pages = storage.get_all_pages().unwrap_or_default();
            let items: Vec<serde_json::Value> = all_pages
                .iter()
                .filter(|page| page.page_type == PageType::Item)
                .filter(|page| {
                    if show_drafts {
                        true
                    } else {
                        // 默认不显示草稿（通过 slug 中的 _draft 后缀判断）
                        !page.slug.ends_with("_draft")
                    }
                })
                .map(|page| {
                    // 检查是否是草稿
                    let is_draft = page.slug.ends_with("_draft");
                    let category = page.category.clone().unwrap_or_default();

                    serde_json::json!({
                        "slug": page.slug,
                        "title": page.title,
                        "category": category,
                        "is_draft": is_draft,
                        "updated_at": page.updated_at
                    })
                })
                .collect();

            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "items": items,
                "count": items.len()
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 获取单篇文章
pub async fn get_item(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let slug = path.into_inner();
    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    match StorageDB::new(&storage_path) {
        Ok(storage) => match storage.get_page(&slug) {
            Ok(Some(page)) => {
                let is_draft = slug.ends_with("_draft");
                let category = page.category.clone().unwrap_or_default();

                HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "item": {
                        "slug": page.slug,
                        "title": page.title,
                        "content": page.content, // Note: This is HTML, need to store raw markdown
                        "category": category,
                        "is_draft": is_draft,
                        "updated_at": page.updated_at
                    }
                }))
            }
            Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
                "status": "error",
                "message": "文章不存在"
            })),
            Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": e.to_string()
            })),
        },
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 创建分类
pub async fn create_category(
    req: actix_web::HttpRequest,
    body: web::Json<CreateCategoryRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let request = body.into_inner();
    let slug = request.name.to_lowercase().replace(' ', "-");

    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    match StorageDB::new(&storage_path) {
        Ok(storage) => {
            // 检查分类是否已存在
            if storage.get_page(&slug).unwrap_or(None).is_some() {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "status": "error",
                    "message": "分类已存在"
                }));
            }

            // 创建分类目录
            let category_dir = config.paths.content_dir.join(&request.name);
            if let Err(e) = fs::create_dir_all(&category_dir) {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": format!("创建分类目录失败: {}", e)
                }));
            }

            // 创建 index.md
            let index_content = format!(
                r#"---
description: {}
---

# {}

这里是 {} 的介绍内容。
"#,
                request.description.clone().unwrap_or_default(),
                request.name,
                request.name
            );
            let index_path = category_dir.join("index.md");
            if let Err(e) = fs::write(&index_path, &index_content) {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "status": "error",
                    "message": format!("创建分类索引文件失败: {}", e)
                }));
            }

            // 生成 HTML
            let mut scanner = Scanner::new(config.paths.content_dir.clone());
            let site_content = match scanner.scan() {
                Ok(content) => content,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("扫描内容失败: {}", e)
                    }));
                }
            };

            // 找到新创建的分类
            if let Some(category) = site_content.categories.iter().find(|c| c.url == slug) {
                let renderer = match crate::templates::TemplateRenderer::new(&config) {
                    Ok(r) => r,
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(serde_json::json!({
                            "status": "error",
                            "message": format!("创建模板渲染器失败: {}", e)
                        }));
                    }
                };

                let category_html = match renderer.render_category(category) {
                    Ok(html) => html,
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(serde_json::json!({
                            "status": "error",
                            "message": format!("渲染分类页面失败: {}", e)
                        }));
                    }
                };

                let category_page = Page {
                    id: Uuid::new_v4().to_string(),
                    slug: slug.clone(),
                    page_type: PageType::Category,
                    title: request.name.clone(),
                    content: category_html,
                    category: None,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };

                if let Err(e) = storage.save_page(&category_page) {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("保存分类页面失败: {}", e)
                    }));
                }
            }

            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "分类创建成功",
                "slug": slug
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 删除分类
pub async fn delete_category(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let slug = path.into_inner();
    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    match StorageDB::new(&storage_path) {
        Ok(storage) => {
            // 删除数据库中的页面
            storage.delete_page(&slug).ok();

            // 删除文件系统中的内容
            let category_dir = config.paths.content_dir.join(&slug);
            if category_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&category_dir) {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("删除分类目录失败: {}", e)
                    }));
                }
            }

            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "分类删除成功"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        })),
    }
}

/// 创建文章
pub async fn create_item(
    req: actix_web::HttpRequest,
    body: web::Json<CreateItemRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let request = body.into_inner();

    // 验证分类存在
    let category_dir = config.paths.content_dir.join(&request.category);
    if !category_dir.exists() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "分类不存在"
        }));
    }

    // 生成 slug
    let slug = format!("{}-{}", request.category, request.item_name);

    // 如果是草稿，添加 _draft 后缀
    let is_draft = request.is_draft.unwrap_or(false);
    let slug = if is_draft {
        format!("{}_draft", slug)
    } else {
        slug
    };

    // 创建文章目录
    let item_dir = category_dir.join(&request.item_name);
    if let Err(e) = fs::create_dir_all(&item_dir) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("创建文章目录失败: {}", e)
        }));
    }

    // 构建 Markdown 内容
    let frontmatter = format!(
        r#"---
title: {}
date: {}
author: {}
description: {}
tags: {}
draft: {}
---

{}
"#,
        request.title,
        request.date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        request.author.unwrap_or_else(|| config.site.author.clone()),
        request.description.unwrap_or_default(),
        request.tags.clone().unwrap_or_default().join(", "),
        if is_draft { "true" } else { "false" },
        request.content
    );

    // 写入 Markdown 文件
    let md_path = item_dir.join(format!("{}.md", request.item_name));
    if let Err(e) = fs::write(&md_path, &frontmatter) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("写入文章文件失败: {}", e)
        }));
    }

    // 创建 attachment 目录
    let attachment_dir = item_dir.join("attachment");
    if let Err(e) = fs::create_dir_all(&attachment_dir) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("创建附件目录失败: {}", e)
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "文章创建成功",
        "slug": slug
    }))
}

/// 更新文章
pub async fn update_item(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateItemRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let old_slug = path.into_inner();
    let request = body.into_inner();

    // 解析旧 slug 获取分类和文章名
    let parts: Vec<&str> = old_slug.splitn(2, '-').collect();
    if parts.len() < 2 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "无效的文章 slug"
        }));
    }

    let old_category = parts[0];
    let old_item_name = if old_slug.ends_with("_draft") {
        let without_draft = &old_slug[..old_slug.len() - 6];
        let parts2: Vec<&str> = without_draft.splitn(2, '-').collect();
        if parts2.len() >= 2 {
            parts2[1]
        } else {
            parts[1]
        }
    } else {
        parts[1]
    };

    // 确定是否需要移动目录
    let new_category = &request.category;
    let new_item_name = &request.item_name;
    let should_move = old_category != new_category || old_item_name != new_item_name;

    // 如果需要移动，先删除旧目录
    if should_move {
        let old_item_dir = config.paths.content_dir.join(old_category).join(old_item_name);
        if old_item_dir.exists() {
            fs::remove_dir_all(&old_item_dir).ok();
        }
    }

    // 验证分类存在
    let category_dir = config.paths.content_dir.join(new_category);
    if !category_dir.exists() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "分类不存在"
        }));
    }

    // 生成新 slug
    let slug = format!("{}-{}", new_category, new_item_name);
    let is_draft = request.is_draft.unwrap_or(false);
    let slug = if is_draft {
        format!("{}_draft", slug)
    } else {
        slug
    };

    // 创建文章目录
    let item_dir = category_dir.join(new_item_name);
    if let Err(e) = fs::create_dir_all(&item_dir) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("创建文章目录失败: {}", e)
        }));
    }

    // 构建 Markdown 内容
    let frontmatter = format!(
        r#"---
title: {}
date: {}
author: {}
description: {}
tags: {}
draft: {}
---

{}
"#,
        request.title,
        request.date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        request.author.unwrap_or_else(|| config.site.author.clone()),
        request.description.unwrap_or_default(),
        request.tags.clone().unwrap_or_default().join(", "),
        if is_draft { "true" } else { "false" },
        request.content
    );

    // 写入 Markdown 文件
    let md_path = item_dir.join(format!("{}.md", new_item_name));
    if let Err(e) = fs::write(&md_path, &frontmatter) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("写入文章文件失败: {}", e)
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "文章更新成功",
        "slug": slug
    }))
}

/// 删除文章
pub async fn delete_item(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let slug = path.into_inner();

    // 解析 slug
    let is_draft = slug.ends_with("_draft");
    let clean_slug = if is_draft {
        &slug[..slug.len() - 6]
    } else {
        &slug
    };

    let parts: Vec<&str> = clean_slug.splitn(2, '-').collect();
    if parts.len() < 2 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "无效的文章 slug"
        }));
    }

    let category = parts[0];
    let item_name = parts[1];

    // 删除数据库中的页面
    let storage_path = config
        .paths
        .storage_database_path
        .to_string_lossy()
        .to_string();

    if let Ok(storage) = StorageDB::new(&storage_path) {
        storage.delete_page(&slug).ok();
        storage.delete_attachments_by_slug(&slug).ok();
    }

    // 删除文件系统中的内容
    let item_dir = config.paths.content_dir.join(category).join(item_name);
    if item_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&item_dir) {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("删除文章目录失败: {}", e)
            }));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "文章删除成功"
    }))
}

/// 发布草稿
pub async fn publish_draft(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let draft_slug = path.into_inner();

    if !draft_slug.ends_with("_draft") {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "不是草稿"
        }));
    }

    let clean_slug = &draft_slug[..draft_slug.len() - 6];

    // 解析 slug
    let parts: Vec<&str> = clean_slug.splitn(2, '-').collect();
    if parts.len() < 2 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "无效的文章 slug"
        }));
    }

    let category = parts[0];
    let item_name = parts[1];

    // 读取草稿文件
    let draft_path = config.paths.content_dir
        .join(category)
        .join(item_name)
        .join(format!("{}.md", item_name));

    if !draft_path.exists() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "status": "error",
            "message": "草稿文件不存在"
        }));
    }

    // 读取并修改 frontmatter
    if let Ok(content) = fs::read_to_string(&draft_path) {
        // 将 draft: true 改为 false
        let new_content = content.replace("draft: true", "draft: false");

        if let Err(e) = fs::write(&draft_path, &new_content) {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("更新草稿失败: {}", e)
            }));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "草稿已发布",
        "slug": clean_slug.to_string()
    }))
}

/// 导入内容（全量或增量）
pub async fn import_content(
    req: actix_web::HttpRequest,
    body: web::Json<ImportRequest>,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let request = body.into_inner();

    // 导入处理将在实际文件上传时进行
    // 这里返回上传端点信息
    let upload_url = if request.import_type == "full" {
        "/api/admin/upload/full".to_string()
    } else {
        format!(
            "/api/admin/upload/incremental/{}",
            request.target_category.clone().unwrap_or_default()
        )
    };

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "请使用 POST 上传文件到指定端点",
        "upload_url": upload_url,
        "supported_formats": ["zip", "tar.gz"]
    }))
}

/// 处理全量上传
pub async fn upload_full_content(
    config: web::Data<Config>,
) -> impl Responder {
    // 这里的实现需要 multipart 支持
    // 简化版本：返回提示信息
    HttpResponse::Ok().json(serde_json::json!({
        "status": "info",
        "message": "全量上传功能需要通过表单上传 ZIP 文件",
        "endpoint": "POST /api/admin/upload/full",
        "form_field": "file"
    }))
}

/// 处理增量上传
pub async fn upload_incremental_content(
    path: web::Path<String>,
    config: web::Data<Config>,
) -> impl Responder {
    let category = path.into_inner();

    // 验证分类存在
    let category_dir = config.paths.content_dir.join(&category);
    if !category_dir.exists() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "分类不存在"
        }));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "info",
        "message": format!("增量上传到分类: {}", category),
        "endpoint": format!("POST /api/admin/upload/incremental/{}", category),
        "form_field": "file"
    }))
}

/// 上传分类压缩包并创建分类
pub async fn upload_category_package(
    req: actix_web::HttpRequest,
    mut payload: Multipart,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let content_dir = &config.paths.content_dir;

    // 遍历 multipart 数据，找到文件字段
    while let Some(Ok(mut field)) = payload.next().await {
        // 检查字段名是否为 file
        let name = field.name();
        if name == "file" {
            // 获取原始文件名以保留扩展名
            let content_disposition = field.content_disposition();
            let original_filename = content_disposition.get_filename().unwrap_or("upload");
            let extension = std::path::Path::new(original_filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");
            
            // 保存临时文件，保留原始文件扩展名
            let temp_dir = std::env::temp_dir();
            let temp_filename = if extension.is_empty() {
                format!("lf_blog_upload_{}", Uuid::new_v4())
            } else {
                format!("lf_blog_upload_{}.{}", Uuid::new_v4(), extension)
            };
            let temp_file_path = temp_dir.join(temp_filename);

            // 创建临时文件并写入
            let mut temp_file = match fs::File::create(&temp_file_path) {
                Ok(f) => f,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("创建临时文件失败: {}", e)
                    }));
                }
            };

            // 读取并写入文件
            while let Some(Ok(chunk)) = field.next().await {
                if let Err(e) = temp_file.write_all(&chunk) {
                    let _ = fs::remove_file(&temp_file_path);
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("写入临时文件失败: {}", e)
                    }));
                }
            }

            // 解压缩文件
            let result = extract_category_package(&temp_file_path, content_dir);

            // 删除临时文件
            let _ = fs::remove_file(&temp_file_path);

            return result;
        }
    }

    HttpResponse::BadRequest().json(serde_json::json!({
        "status": "error",
        "message": "未找到上传的文件"
    }))
}


/// 解压缩分类包
fn extract_category_package(
    archive_path: &PathBuf,
    content_dir: &PathBuf,
) -> HttpResponse {
    let filename = archive_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");

    let is_zip = filename.to_lowercase().ends_with(".zip") || filename.ends_with(".ZIP");
    let is_tar = filename.to_lowercase().ends_with(".tar.gz") || filename.to_lowercase().ends_with(".tgz");

    // 调试信息：记录实际收到的文件名
    log::debug!("检查分类包格式: 文件名={}, is_zip={}, is_tar={}", filename, is_zip, is_tar);

    if !is_zip && !is_tar {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "不支持的文件格式，请上传 .zip 或 .tar.gz 文件"
        }));
    }

    // 创建临时解压目录
    let temp_extract_dir = archive_path.parent()
        .unwrap_or(&std::env::temp_dir())
        .join(format!("lf_blog_extract_{}", Uuid::new_v4()));

    if let Err(e) = fs::create_dir_all(&temp_extract_dir) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("创建临时目录失败: {}", e)
        }));
    }

    // 解压缩
    let extract_result = if is_zip {
        extract_zip(archive_path, &temp_extract_dir)
    } else {
        extract_tar(archive_path, &temp_extract_dir)
    };

    if let Err(e) = extract_result {
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("解压失败: {}", e)
        }));
    }

    // 查找解压后的分类目录
    let mut extract_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&temp_extract_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    extract_dirs.push(path.clone());
                }
            }
        }
    }

    if extract_dirs.is_empty() {
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "压缩包内容为空"
        }));
    }

    // 如果只有一个目录，使用它作为分类；否则尝试找到 index.md 所在的目录
    let category_dir = if extract_dirs.len() == 1 {
        extract_dirs.remove(0)
    } else {
        // 查找包含 index.md 的目录
        let mut found = None;
        for dir in &extract_dirs {
            if dir.join("index.md").exists() {
                found = Some(dir.clone());
                break;
            }
        }
        if let Some(d) = found {
            d
        } else {
            // 使用第一个目录
            extract_dirs.remove(0)
        }
    };

    // 获取分类名称（目录名）
    let category_name = category_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let target_dir = content_dir.join(&category_name);

    // 如果目标目录已存在，先删除
    if target_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&target_dir) {
            let _ = fs::remove_dir_all(&temp_extract_dir);
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "error",
                "message": format!("目标目录已存在且无法删除: {}", e)
            }));
        }
    }

    // 移动解压后的内容到目标位置
    if let Err(e) = fs::rename(&category_dir, &target_dir) {
        // 如果重命名失败，尝试复制后删除
        if let Err(e2) = copy_dir(&category_dir, &target_dir) {
            let _ = fs::remove_dir_all(&temp_extract_dir);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("移动文件失败: {}，复制也失败: {}", e, e2)
            }));
        }
        let _ = fs::remove_dir_all(&category_dir);
    }

    // 清理临时目录
    let _ = fs::remove_dir_all(&temp_extract_dir);

    // 确保 attachment 目录存在
    let attachment_dir = target_dir.join("attachment");
    if !attachment_dir.exists() {
        let _ = fs::create_dir_all(&attachment_dir);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": format!("分类 '{}' 创建成功", category_name),
        "category": category_name
    }))
}

/// 上传文章压缩包到指定分类
pub async fn upload_item_package(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    mut payload: Multipart,
    config: web::Data<Config>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let category_slug = path.into_inner();
    let content_dir = &config.paths.content_dir;
    let category_dir = content_dir.join(&category_slug);

    // 验证分类存在
    if !category_dir.exists() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "分类不存在"
        }));
    }

    // 遍历 multipart 数据，找到文件字段
    while let Some(Ok(mut field)) = payload.next().await {
        // 检查字段名是否为 file
        let name = field.name();
        if name == "file" {
            // 获取原始文件名以保留扩展名
            let content_disposition = field.content_disposition();
            let original_filename = content_disposition.get_filename().unwrap_or("upload");
            let extension = std::path::Path::new(original_filename)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");
            
            // 保存临时文件，保留原始文件扩展名
            let temp_dir = std::env::temp_dir();
            let temp_filename = if extension.is_empty() {
                format!("lf_blog_upload_{}", Uuid::new_v4())
            } else {
                format!("lf_blog_upload_{}.{}", Uuid::new_v4(), extension)
            };
            let temp_file_path = temp_dir.join(temp_filename);

            // 创建临时文件并写入
            let mut temp_file = match fs::File::create(&temp_file_path) {
                Ok(f) => f,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("创建临时文件失败: {}", e)
                    }));
                }
            };

            // 读取并写入文件
            while let Some(Ok(chunk)) = field.next().await {
                if let Err(e) = temp_file.write_all(&chunk) {
                    let _ = fs::remove_file(&temp_file_path);
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": format!("写入临时文件失败: {}", e)
                    }));
                }
            }

            // 解压缩文件
            let result = extract_item_package(&temp_file_path, &category_dir);

            // 删除临时文件
            let _ = fs::remove_file(&temp_file_path);

            return result;
        }
    }

    HttpResponse::BadRequest().json(serde_json::json!({
        "status": "error",
        "message": "未找到上传的文件"
    }))
}
/// 解压缩文章包
fn extract_item_package(
    archive_path: &PathBuf,
    category_dir: &PathBuf,
) -> HttpResponse {
    let filename = archive_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");

    let is_zip = filename.to_lowercase().ends_with(".zip") || filename.ends_with(".ZIP");
    let is_tar = filename.to_lowercase().ends_with(".tar.gz") || filename.to_lowercase().ends_with(".tgz");

    // 调试信息：记录实际收到的文件名
    log::debug!("检查文章包格式: 文件名={}, is_zip={}, is_tar={}", filename, is_zip, is_tar);

    if !is_zip && !is_tar {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "不支持的文件格式，请上传 .zip 或 .tar.gz 文件"
        }));
    }

    // 创建临时解压目录
    let temp_extract_dir = archive_path.parent()
        .unwrap_or(&std::env::temp_dir())
        .join(format!("lf_blog_extract_{}", Uuid::new_v4()));

    if let Err(e) = fs::create_dir_all(&temp_extract_dir) {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("创建临时目录失败: {}", e)
        }));
    }

    // 解压缩
    let extract_result = if is_zip {
        extract_zip(archive_path, &temp_extract_dir)
    } else {
        extract_tar(archive_path, &temp_extract_dir)
    };

    if let Err(e) = extract_result {
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "status": "error",
            "message": format!("解压失败: {}", e)
        }));
    }

    // 查找解压后的文章目录
    let mut extract_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&temp_extract_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    extract_dirs.push(path.clone());
                }
            }
        }
    }

    if extract_dirs.is_empty() {
        let _ = fs::remove_dir_all(&temp_extract_dir);
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "压缩包内容为空"
        }));
    }

    // 如果只有一个目录，使用它作为文章；否则尝试找到 .md 文件所在的目录
    let item_dir = if extract_dirs.len() == 1 {
        extract_dirs.remove(0)
    } else {
        // 查找包含 .md 文件的目录
        let mut found = None;
        for dir in &extract_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().map(|e| e.to_str()).flatten() == Some("md") {
                            found = Some(dir.clone());
                            break;
                        }
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        if let Some(d) = found {
            d
        } else {
            // 使用第一个目录
            extract_dirs.remove(0)
        }
    };

    // 获取文章名称（目录名）
    let item_name = item_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let target_dir = category_dir.join(&item_name);

    // 如果目标目录已存在，先删除
    if target_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&target_dir) {
            let _ = fs::remove_dir_all(&temp_extract_dir);
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "error",
                "message": format!("目标目录已存在且无法删除: {}", e)
            }));
        }
    }

    // 移动解压后的内容到目标位置
    if let Err(e) = fs::rename(&item_dir, &target_dir) {
        // 如果重命名失败，尝试复制后删除
        if let Err(e2) = copy_dir(&item_dir, &target_dir) {
            let _ = fs::remove_dir_all(&temp_extract_dir);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("移动文件失败: {}，复制也失败: {}", e, e2)
            }));
        }
        let _ = fs::remove_dir_all(&item_dir);
    }

    // 清理临时目录
    let _ = fs::remove_dir_all(&temp_extract_dir);

    // 确保 attachment 目录存在
    let attachment_dir = target_dir.join("attachment");
    if !attachment_dir.exists() {
        let _ = fs::create_dir_all(&attachment_dir);
    }

    // 确保有 index.md 或同名的 .md 文件
    let md_files: Vec<_> = match target_dir.read_dir() {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ex| ex.to_str()).flatten() == Some("md"))
            .collect(),
        Err(_) => Vec::new(),
    };

    if md_files.is_empty() {
        // 创建默认的 index.md
        let default_content = format!(
            r#"---
title: {}
date: {}
author: {}
description: ""
tags: []
draft: false
---

# {}

这里是文章的默认内容。
"#,
            item_name,
            chrono::Utc::now().format("%Y-%m-%d").to_string(),
            "Author",
            item_name
        );
        let md_path = target_dir.join(format!("{}.md", item_name));
        let _ = fs::write(&md_path, &default_content);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": format!("文章 '{}' 创建成功", item_name),
        "item": item_name
    }))
}

/// 解压 ZIP 文件
fn extract_zip(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = dest_dir.join(entry.name());

        if entry.name().ends_with('/') {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut out_file = fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    Ok(())
}

/// 解压 TAR.GZ 文件
fn extract_tar(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(archive_path)?;

    // 解压 gzip
    let gzip_decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gzip_decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let out_path = dest_dir.join(entry.path()?);

        if entry.header().entry_type() == tar::EntryType::Directory {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            entry.unpack(&out_path)?;
        }
    }
    Ok(())
}

/// 复制目录
fn copy_dir(src: &PathBuf, dst: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// 导出内容
pub async fn export_content(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
    query: web::Query<Option<String>>,
) -> impl Responder {
    // 验证管理员身份
    if !require_auth(&req, &config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "未授权访问，请先登录"
        }));
    }
    let category_filter = query.into_inner();

    // 收集要导出的文件
    let mut files = Vec::new();
    let mut total_size = 0;

    if let Some(category) = category_filter {
        // 导出指定分类
        let category_dir = config.paths.content_dir.join(&category);
        if category_dir.exists() {
            collect_files(&category_dir, &mut files, &mut total_size, &config.paths.content_dir);
        }
    } else {
        // 导出全部内容
        collect_files(&config.paths.content_dir, &mut files, &mut total_size, &config.paths.content_dir);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "message": "内容导出信息",
        "file_count": files.len(),
        "data_size": total_size,
        "files": files.iter().take(50).collect::<Vec<_>>(), // 只返回前50个文件信息
        "truncated": files.len() > 50
    }))
}

/// 收集目录中的文件
fn collect_files(dir: &PathBuf, files: &mut Vec<String>, total_size: &mut usize, content_dir: &PathBuf) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(metadata) = fs::metadata(&path) {
                        *total_size += metadata.len() as usize;
                    }
                    if let Some(rel_path) = path.strip_prefix(content_dir).ok() {
                        files.push(rel_path.to_string_lossy().to_string());
                    }
                } else if path.is_dir() {
                    collect_files(&path, files, total_size, content_dir);
                }
            }
        }
    }
}

/// 配置管理员路由
pub fn configure_admin_routes(cfg: &mut web::ServiceConfig) {
    // 公开路由
    cfg.route("/api/admin/login", web::post().to(admin_login));

    // 需要认证的路由（使用简单 token 验证）
    cfg.route("/api/admin/overview", web::get().to(get_admin_overview))
        .route("/api/admin/categories", web::get().to(get_categories))
        .route("/api/admin/items", web::get().to(get_items))
        .route("/api/admin/items/{slug}", web::get().to(get_item))
        .route("/api/admin/categories", web::post().to(create_category))
        .route("/api/admin/categories/{slug}", web::delete().to(delete_category))
        .route("/api/admin/items", web::post().to(create_item))
        .route("/api/admin/items/{slug}", web::put().to(update_item))
        .route("/api/admin/items/{slug}", web::delete().to(delete_item))
        .route("/api/admin/items/{slug}/publish", web::post().to(publish_draft))
        .route("/api/admin/import", web::post().to(import_content))
        .route("/api/admin/export", web::get().to(export_content))
        // 上传压缩包路由
        .route("/api/admin/upload/category", web::post().to(upload_category_package))
        .route("/api/admin/upload/item/{category}", web::post().to(upload_item_package));

// 页面路由
cfg.route("/admin/login", web::get().to(admin_login_page))
.route("/admin/login", web::post().to(admin_login_handler))
.route("/admin/logout", web::get().to(admin_logout))
.route("/admin", web::get().to(admin_overview_page))
.route("/admin/", web::get().to(admin_overview_page))
.route("/admin/categories", web::get().to(admin_categories_page))
.route("/admin/categories", web::post().to(admin_create_category_handler))
.route("/admin/categories/new", web::get().to(admin_new_category_page))
.route("/admin/categories/new", web::post().to(admin_create_category_handler))
.route("/admin/categories/{slug}/delete", web::post().to(admin_delete_category_handler))
.route("/admin/items", web::get().to(admin_items_page))
.route("/admin/items", web::post().to(admin_create_item_handler))
.route("/admin/items/new", web::get().to(admin_new_item_page))
.route("/admin/items/new", web::post().to(admin_create_item_handler))
.route("/admin/items/{slug}/edit", web::get().to(admin_edit_item_page))
.route("/admin/items/{slug}", web::post().to(admin_update_item_handler))
.route("/admin/items/{slug}/edit", web::post().to(admin_update_item_handler))
        .route("/admin/items/{slug}/publish", web::post().to(admin_publish_draft_handler))
        .route("/admin/items/{slug}/delete", web::post().to(admin_delete_item_handler))
        .route("/admin/analytics", web::get().to(admin_analytics_page))
        .route("/admin/compile", web::get().to(admin_compile_page));
}

// ==================== 页面路由处理器 ====================

/// 获取登录 cookie 中的 token
fn get_admin_token(req: &actix_web::HttpRequest) -> Option<String> {
    req.cookie("admin_token").map(|c| c.value().to_string())
}

/// 验证并获取存储数据库连接
fn get_storage(config: &Config) -> Result<StorageDB, actix_web::Error> {
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    StorageDB::new(&storage_path).map_err(|e| {
        actix_web::error::ErrorInternalServerError(e.to_string())
    })
}

/// 创建模板渲染器
fn get_renderer(config: &Config) -> Result< crate::templates::TemplateRenderer, actix_web::Error> {
    crate::templates::TemplateRenderer::new(config).map_err(|e| {
        actix_web::error::ErrorInternalServerError(e.to_string())
    })
}

/// 检查是否已登录
fn require_auth(req: &actix_web::HttpRequest, config: &Config) -> bool {
    if let Some(token) = get_admin_token(req) {
        verify_admin_token(config, &token)
    } else {
        false
    }
}

/// 管理后台登录页面
pub async fn admin_login_page(
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_login(None)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 管理后台登录处理
pub async fn admin_login_handler(
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    let password = form.get("password").cloned().unwrap_or_default();
    let renderer = get_renderer(&config)?;

    if verify_admin_password(&config, &password) {
        let token = generate_token();
        let html = renderer.render_admin_overview(
            &serde_json::json!({
                "total_categories": 0,
                "total_items": 0,
                "total_attachments": 0,
                "total_comments": 0
            }),
            None,
            Some("登录成功"),
            true,
        )?;
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .cookie(actix_web::cookie::Cookie::build("admin_token", &token)
                .path("/")
                .max_age(actix_web::cookie::time::Duration::hours(24))
                .finish())
            .body(html))
    } else {
        let html = renderer.render_admin_login(Some("密码错误"))?;
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html))
    }
}

/// 退出登录
pub async fn admin_logout(config: web::Data<Config>) -> actix_web::Result<HttpResponse> {
    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_login(None)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .cookie(actix_web::cookie::Cookie::build("admin_token", "")
            .path("/")
            .max_age(actix_web::cookie::time::Duration::seconds(0))
            .finish())
        .body(html))
}

/// 管理后台访问统计页面
pub async fn admin_analytics_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_analytics(None, true)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 管理后台仪表盘页面
pub async fn admin_overview_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        let renderer = get_renderer(&config)?;
        let html = renderer.render_admin_login(None)?;
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(html));
    }

    let storage = get_storage(&config).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let stats = storage.get_stats().unwrap_or_else(|_| crate::storage::SiteStats {
        total_categories: 0,
        total_items: 0,
        total_attachments: 0,
        last_compiled: None,
    });
    let last_compiled = storage.get_last_compiled().ok().flatten();

    // 获取评论统计
    let comments_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let comment_stats = crate::comments::CommentDB::new(&comments_path)
        .and_then(|db| db.get_comment_stats())
        .unwrap_or_default();
    let total_comments = comment_stats.iter().map(|(_, count)| count).sum::<i64>();

    let overview = serde_json::json!({
        "total_categories": stats.total_categories,
        "total_items": stats.total_items,
        "total_attachments": stats.total_attachments,
        "total_comments": total_comments
    });

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_overview(&overview, last_compiled.as_deref(), None, true)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 管理后台分类列表页面
pub async fn admin_categories_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let storage = get_storage(&config).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let category_pages = storage.get_pages_by_type(PageType::Category).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let categories: Vec<serde_json::Value> = category_pages
        .iter()
        .map(|page| {
            let items = storage.get_items_by_category(&page.slug).unwrap_or_default();
            serde_json::json!({
                "slug": page.slug.clone(),
                "title": page.title.clone(),
                "item_count": items.len()
            })
        })
        .collect();

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_categories(&categories, None, true)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 新建分类页面
pub async fn admin_new_category_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_new_category()?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 创建分类处理
pub async fn admin_create_category_handler(
    req: actix_web::HttpRequest,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let name = form.get("name").cloned().unwrap_or_default();
    let description = form.get("description").cloned();

    if name.is_empty() {
        let renderer = get_renderer(&config)?;
        let html = renderer.render_admin_categories(&[], Some("分类名称不能为空"), false)?;
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html));
    }

    // 创建分类目录
    let slug = name.to_lowercase().replace(' ', "-");
    let category_dir = config.paths.content_dir.join(&name);

    if category_dir.exists() {
        let renderer = get_renderer(&config)?;
        let html = renderer.render_admin_categories(&[], Some("分类已存在"), false)?;
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html));
    }

    if let Err(e) = std::fs::create_dir_all(&category_dir) {
        let renderer = get_renderer(&config)?;
        let html = renderer.render_admin_categories(&[], Some(&format!("创建分类目录失败: {}", e)), false)?;
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html));
    }

    // 创建 index.md
    let index_content = format!(
        r#"---
description: {}
---

# {}

这里是 {} 的介绍内容.
"#,
        description.clone().unwrap_or_default(),
        name,
        name
    );
    let index_path = category_dir.join("index.md");
    if let Err(e) = std::fs::write(&index_path, &index_content) {
        let renderer = get_renderer(&config)?;
        let html = renderer.render_admin_categories(&[], Some(&format!("创建分类索引文件失败: {}", e)), false)?;
        return Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html));
    }

    Ok(HttpResponse::SeeOther()
        .append_header((actix_web::http::header::LOCATION, "/admin/categories"))
        .body(String::new()))
}

/// 删除分类处理
pub async fn admin_delete_category_handler(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let slug = path.into_inner();
    let _ = delete_category(req, web::Path::from(slug), config).await;

    Ok(HttpResponse::SeeOther()
        .append_header((actix_web::http::header::LOCATION, "/admin/categories"))
        .body(String::new()))
}

/// 管理后台文章列表页面
pub async fn admin_items_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let show_drafts = query.get("drafts").map(|v| v == "true").unwrap_or(false);
    let storage = get_storage(&config)?;
    let all_pages = storage.get_all_pages().unwrap_or_default();

    let items: Vec<serde_json::Value> = all_pages
        .iter()
        .filter(|page| page.page_type == PageType::Item)
        .filter(|page| {
            if show_drafts {
                true
            } else {
                !page.slug.ends_with("_draft")
            }
        })
        .map(|page| {
            let is_draft = page.slug.ends_with("_draft");
            let category = page.category.clone().unwrap_or_default();
            serde_json::json!({
                "slug": page.slug,
                "title": page.title,
                "category": category,
                "is_draft": is_draft,
                "updated_at": page.updated_at
            })
        })
        .collect();

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_items(&items, show_drafts, None, true)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 新建文章页面
pub async fn admin_new_item_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let storage = get_storage(&config).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let category_pages = storage.get_pages_by_type(PageType::Category).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let categories: Vec<serde_json::Value> = category_pages
        .iter()
        .map(|page| {
            serde_json::json!({
                "slug": page.slug.clone(),
                "title": page.title.clone()
            })
        })
        .collect();

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_new_item(&categories)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// 创建文章处理
pub async fn admin_create_item_handler(
    req: actix_web::HttpRequest,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let request = CreateItemRequest {
        category: form.get("category").cloned().unwrap_or_default(),
        item_name: form.get("item_name").cloned().unwrap_or_default(),
        title: form.get("title").cloned().unwrap_or_default(),
        content: form.get("content").cloned().unwrap_or_default(),
        date: form.get("date").cloned(),
        author: form.get("author").cloned(),
        description: form.get("description").cloned(),
        tags: form.get("tags").map(|t| t.split(',').map(|s| s.trim().to_string()).collect()),
        is_draft: form.get("is_draft").map(|v| v == "true"),
    };

    let _ = create_item(req, web::Json(request), config).await;

    Ok(HttpResponse::SeeOther()
        .append_header((actix_web::http::header::LOCATION, "/admin/items"))
        .body(String::new()))
}

/// 编辑文章页面
pub async fn admin_edit_item_page(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let slug = path.into_inner();
    let storage = get_storage(&config).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    match storage.get_page(&slug) {
        Ok(Some(page)) => {
            let is_draft = slug.ends_with("_draft");
            let clean_slug = if is_draft { &slug[..slug.len() - 6] } else { &slug };
            let parts: Vec<&str> = clean_slug.splitn(2, '-').collect();
            let item_name = if parts.len() >= 2 { parts[1].to_string() } else { slug.clone() };

            let category = page.category.clone().unwrap_or_default();

            // 简化标签获取
            let tags = String::new();

            let item = serde_json::json!({
                "slug": page.slug,
                "title": page.title,
                "category": category,
                "item_name": item_name,
                "content": page.content,
                "is_draft": is_draft,
                "date": "",
                "author": "",
                "description": "",
                "tags": tags,
                "updated_at": page.updated_at
            });

            let category_pages = storage.get_pages_by_type(PageType::Category).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
            let categories: Vec<serde_json::Value> = category_pages
                .iter()
                .map(|page| {
                    serde_json::json!({
                        "slug": page.slug.clone(),
                        "title": page.title.clone()
                    })
                })
                .collect();

            let renderer = get_renderer(&config)?;
            let html = renderer.render_admin_edit_item(&item, &categories)?;
            Ok(HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(html))
        }
        Ok(None) => {
            Ok(HttpResponse::NotFound()
                .content_type("text/html; charset=utf-8")
                .body("文章不存在"))
        }
        Err(e) => {
            Err(actix_web::error::ErrorInternalServerError(e.to_string()))
        }
    }
}

/// 更新文章处理
pub async fn admin_update_item_handler(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    form: actix_web::web::Form<std::collections::HashMap<String, String>>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let request = UpdateItemRequest {
        category: form.get("category").cloned().unwrap_or_default(),
        item_name: form.get("item_name").cloned().unwrap_or_default(),
        title: form.get("title").cloned().unwrap_or_default(),
        content: form.get("content").cloned().unwrap_or_default(),
        date: form.get("date").cloned(),
        author: form.get("author").cloned(),
        description: form.get("description").cloned(),
        tags: form.get("tags").map(|t| t.split(',').map(|s| s.trim().to_string()).collect()),
        is_draft: form.get("is_draft").map(|v| v == "true"),
    };

    let _ = update_item(req, path, web::Json(request), config).await;

    Ok(HttpResponse::SeeOther()
        .append_header((actix_web::http::header::LOCATION, "/admin/items"))
        .body(String::new()))
}

/// 发布草稿处理
pub async fn admin_publish_draft_handler(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let _ = publish_draft(req, path, config).await;

    Ok(HttpResponse::SeeOther()
        .append_header((actix_web::http::header::LOCATION, "/admin/items"))
        .body(String::new()))
}

/// 删除文章处理
pub async fn admin_delete_item_handler(
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let _ = delete_item(req, path, config).await;

    Ok(HttpResponse::SeeOther()
        .append_header((actix_web::http::header::LOCATION, "/admin/items"))
        .body(String::new()))
}

/// 编译发布页面
pub async fn admin_compile_page(
    req: actix_web::HttpRequest,
    config: web::Data<Config>,
) -> actix_web::Result<HttpResponse> {
    if !require_auth(&req, &config) {
        return Ok(HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, "/admin/login"))
            .body(String::new()));
    }

    let renderer = get_renderer(&config)?;
    let html = renderer.render_admin_compile(None, true)?;
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}