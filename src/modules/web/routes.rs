use actix_web::{web, HttpResponse, Responder};
use crate::config::Config;
use crate::compiler::Compiler;
use crate::storage::StorageDB;
use crate::admin;
use crate::analytics;
use crate::comments;
use crate::recommender;
use crate::templates::TemplateRenderer;
use crate::scanner::{SiteContent, Category, ContentItem};
use serde_json;

/// Serve pages from database
pub async fn serve_page(path: web::Path<String>, config: web::Data<Config>) -> impl Responder {
    let requested_path = path.into_inner();

    // Skip attachment paths (they should be handled by serve_attachment)
    if requested_path.starts_with("attachment/") {
        return HttpResponse::NotFound().body("Attachment not found");
    }

    // Clean the path - remove leading/trailing slashes and prevent directory traversal
    let clean_path = requested_path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .replace("..", "");

    // Determine the slug (remove .html extension if present)
    let slug = if clean_path.ends_with(".html") {
        clean_path.strip_suffix(".html").unwrap_or(&clean_path).to_string()
    } else {
        clean_path
    };

    // Create storage connection
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Try to get page from database
    match storage.get_page(&slug) {
        Ok(Some(page)) => {
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(page.content)
        }
        Ok(None) => {
            log::debug!("Page not found in database: {}", slug);
            HttpResponse::NotFound().body("Page not found")
        }
        Err(e) => {
            log::error!("Failed to get page from database: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Serve attachments from database
pub async fn serve_attachment(path: web::Path<String>, config: web::Data<Config>) -> impl Responder {
    let filename = path.into_inner();
    log::debug!("Trying to serve attachment: {}", filename);

    // Create storage connection
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Try to get attachment from database
    match storage.get_attachment(&filename) {
        Ok(Some(attachment)) => {
            let mime_type = attachment.mime_type.as_str();
            log::debug!("Serving attachment: {} ({}, {} bytes)", filename, mime_type, attachment.file_size);
            HttpResponse::Ok()
                .content_type(mime_type)
                .body(attachment.file_data)
        }
        Ok(None) => {
            log::debug!("Attachment not found in database: {}", filename);
            HttpResponse::NotFound().body("Attachment not found")
        }
        Err(e) => {
            log::error!("Failed to get attachment from database: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Serve index page from database
pub async fn index(config: web::Data<Config>) -> impl Responder {
    // Create storage connection
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };

    // Try to get index page from database
    match storage.get_page("index") {
        Ok(Some(page)) => {
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(page.content)
        }
        Ok(None) => {
            HttpResponse::NotFound().body("Index page not found. Please compile the site first.")
        }
        Err(e) => {
            log::error!("Failed to get index page from database: {}", e);
            HttpResponse::InternalServerError().body("Database error")
        }
    }
}

/// Serve content files based on URL pattern
pub async fn serve_content(path: web::Path<String>, config: web::Data<Config>) -> impl Responder {
    serve_page(path, config).await
}

/// Trigger recompilation
pub async fn recompile(config: web::Data<Config>) -> impl Responder {
    log::info!("Received recompile request");

    match Compiler::new(config.get_ref().clone()) {
        Ok(mut compiler) => {
            match compiler.compile() {
                Ok(result) => {
                    log::info!("Compilation successful: {} pages, {} attachments",
                        result.total_categories + result.total_items + 1, result.total_attachments);
                    HttpResponse::Ok().json(serde_json::json!({
                        "status": "success",
                        "message": format!("Compiled {} pages, {} attachments",
                            result.total_categories + result.total_items + 1, result.total_attachments),
                        "categories": result.total_categories,
                        "items": result.total_items,
                        "attachments": result.total_attachments
                    }))
                }
                Err(e) => {
                    log::error!("Compilation failed: {}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": e.to_string()
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to create compiler: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": e.to_string()
            }))
        }
    }
}

/// Health check endpoint
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "lf_blog"
    }))
}

/// Get comments for a specific page
pub async fn get_comments(path: web::Path<String>, config: web::Data<Config>) -> impl Responder {
    let slug = path.into_inner();
    log::debug!("Getting comments for slug: {}", slug);

    // Create comments database connection
    let comments_db_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let comments_db = match comments::CommentDB::new(&comments_db_path) {
        Ok(db) => db,
        Err(e) => {
            log::error!("Failed to open comments database: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Database error"
            }));
        }
    };

    match comments_db.get_comments_by_slug(&slug) {
        Ok(comments) => {
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "comments": comments,
                "count": comments.len()
            }))
        }
        Err(e) => {
            log::error!("Failed to get comments: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to retrieve comments"
            }))
        }
    }
}

/// Add a new comment
pub async fn add_comment(
    body: web::Json<comments::CreateCommentRequest>,
    config: web::Data<Config>
) -> impl Responder {
    let request = body.into_inner();
    log::debug!("Adding comment for slug: {}", request.slug);

    // Validate input
    if request.author.trim().is_empty() || request.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "Author and content are required"
        }));
    }

    // Create comments database connection
    let comments_db_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let comments_db = match comments::CommentDB::new(&comments_db_path) {
        Ok(db) => db,
        Err(e) => {
            log::error!("Failed to open comments database: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Database error"
            }));
        }
    };

    match comments_db.add_comment(request) {
        Ok(comment) => {
            log::info!("Comment added successfully: {}", comment.id);
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "Comment added successfully",
                "comment": comment
            }))
        }
        Err(e) => {
            log::error!("Failed to add comment: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to add comment"
            }))
        }
    }
}

/// Get content recommendations
pub async fn get_recommendations(
    path: web::Path<String>,
    config: web::Data<Config>
) -> impl Responder {
    let current_url = path.into_inner();
    log::debug!("Getting recommendations for URL: {}", current_url);

    // Create storage connection to get site content
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Database error"
            }));
        }
    };

    // Get all pages from storage and convert to SiteContent
    let pages = match storage.get_all_pages() {
        Ok(pages) => pages,
        Err(e) => {
            log::error!("Failed to get pages from storage: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to retrieve site content"
            }));
        }
    };

    // Convert pages to SiteContent structure for recommender
    let site_content = convert_pages_to_site_content(&pages);

    // Create recommender and get recommendations
    let recommender = recommender::Recommender::new(site_content);
    let recommendations = recommender.get_recommendations(&current_url, 5);

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "recommendations": recommendations,
        "count": recommendations.len()
    }))
}

/// Get popular content
pub async fn get_popular_content(config: web::Data<Config>) -> impl Responder {
    log::debug!("Getting popular content");

    // Create storage connection to get site content
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Database error"
            }));
        }
    };

    // Get all pages from storage and convert to SiteContent
    let pages = match storage.get_all_pages() {
        Ok(pages) => pages,
        Err(e) => {
            log::error!("Failed to get pages from storage: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to retrieve site content"
            }));
        }
    };

    // Convert pages to SiteContent structure for recommender
    let site_content = convert_pages_to_site_content(&pages);

    // Create recommender and get popular items
    let recommender = recommender::Recommender::new(site_content);
    let popular_items = recommender.get_popular_items(10);

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "popular_items": popular_items,
        "count": popular_items.len()
    }))
}

/// Get latest content
pub async fn get_latest_content(config: web::Data<Config>) -> impl Responder {
    log::debug!("Getting latest content");

    // Create storage connection to get site content
    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Database error"
            }));
        }
    };

    // Get all pages from storage and convert to SiteContent
    let pages = match storage.get_all_pages() {
        Ok(pages) => pages,
        Err(e) => {
            log::error!("Failed to get pages from storage: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to retrieve site content"
            }));
        }
    };

    // Convert pages to SiteContent structure for recommender
    let site_content = convert_pages_to_site_content(&pages);

    // Create recommender and get latest items
    let recommender = recommender::Recommender::new(site_content);
    let latest_items = recommender.get_latest_items(10);

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "latest_items": latest_items,
        "count": latest_items.len()
    }))
}

/// Search content API endpoint
pub async fn search_content(
    query: web::Query<std::collections::HashMap<String, String>>,
    config: web::Data<Config>
) -> impl Responder {
    let search_query = query.get("q").cloned().unwrap_or_default();

    if search_query.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "Search query is empty"
        }));
    }

    log::debug!("Searching for: {}", search_query);

    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Database error"
            }));
        }
    };

    let limit = query.get("limit").and_then(|s| s.parse().ok()).unwrap_or(20);
    let pages = match storage.search_pages(&search_query, limit) {
        Ok(pages) => pages,
        Err(e) => {
            log::error!("Search failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Search failed"
            }));
        }
    };

    let total_count = match storage.search_pages_count(&search_query) {
        Ok(count) => count,
        Err(_) => pages.len(),
    };

    let results: Vec<serde_json::Value> = pages.iter().map(|page| {
        let snippet = extract_snippet(&page.content, &search_query);
        serde_json::json!({
            "slug": page.slug,
            "title": page.title,
            "category": page.category,
            "snippet": snippet,
            "page_type": match page.page_type {
                crate::storage::PageType::Category => "category",
                crate::storage::PageType::Item => "item",
                _ => "unknown"
            }
        })
    }).collect();

    HttpResponse::Ok().json(serde_json::json!({
        "status": "success",
        "query": search_query,
        "results": results,
        "count": results.len(),
        "total": total_count
    }))
}

/// Extract a snippet from content around the search term
fn extract_snippet(content: &str, query: &str) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    if let Some(pos) = content_lower.find(&query_lower) {
        let char_indices: Vec<(usize, char)> = content.char_indices().collect();
        let total_chars = char_indices.len();

        if total_chars == 0 {
            return String::new();
        }

        let pos_chars = content_lower.chars().take(pos).count();
        let query_chars = query.chars().count();

        let start_char = if pos_chars > 40 { pos_chars - 40 } else { 0 };
        let end_char = (pos_chars + query_chars + 60).min(total_chars);

        let start_byte = char_indices[start_char].0;
        let end_byte = if end_char >= total_chars {
            content.len()
        } else {
            char_indices[end_char].0
        };

        let mut snippet = content[start_byte..end_byte].to_string();

        if start_char > 0 {
            snippet = format!("...{}", snippet);
        }
        if end_char < total_chars {
            snippet = format!("{}...", snippet);
        }

        snippet
    } else {
        let chars: Vec<char> = content.chars().collect();
        let snippet: String = chars.iter().take(100).collect();
        if chars.len() > 100 {
            format!("{}...", snippet)
        } else {
            snippet
        }
    }
}

/// Serve search page
pub async fn search_page(
    query: web::Query<std::collections::HashMap<String, String>>,
    config: web::Data<Config>
) -> impl Responder {
    let search_query = query.get("q").cloned().unwrap_or_default();

    let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
    let storage = match StorageDB::new(&storage_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to open storage database: {}", e);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };

    let pages = match storage.search_pages(&search_query, 50) {
        Ok(pages) => pages,
        Err(e) => {
            log::error!("Search failed: {}", e);
            return HttpResponse::InternalServerError().body("Search failed");
        }
    };

    let total_count = match storage.search_pages_count(&search_query) {
        Ok(count) => count,
        Err(_) => pages.len(),
    };

    let results: Vec<serde_json::Value> = pages.iter().map(|page| {
        let snippet = extract_snippet(&page.content, &search_query);
        serde_json::json!({
            "slug": page.slug,
            "title": page.title,
            "category": page.category,
            "snippet": snippet,
            "page_type": match page.page_type {
                crate::storage::PageType::Category => "category",
                crate::storage::PageType::Item => "item",
                _ => "unknown"
            }
        })
    }).collect();

    let site_content = match storage.get_all_pages() {
        Ok(pages) => convert_pages_to_site_content(&pages),
        Err(_) => SiteContent { categories: Vec::new() },
    };

    let mut ctx = tera::Context::new();
    ctx.insert("config", &config);
    ctx.insert("site_content", &site_content);
    ctx.insert("search_query", &search_query);
    ctx.insert("search_results", &results);
    ctx.insert("total_count", &total_count);

    match TemplateRenderer::new(&config) {
        Ok(renderer) => {
            match renderer.render_with_context("search.html", &ctx) {
                Ok(html) => HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(html),
                Err(e) => {
                    log::error!("Failed to render search template: {}", e);
                    HttpResponse::InternalServerError().body("Template error")
                }
            }
        }
        Err(e) => {
            log::error!("Failed to create template renderer: {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}

/// Helper function to convert StorageDB pages to SiteContent for recommender
fn convert_pages_to_site_content(pages: &[crate::storage::Page]) -> SiteContent {
    use crate::storage::PageType;
    
    let mut categories: std::collections::HashMap<String, Vec<&crate::storage::Page>> = std::collections::HashMap::new();
    
    // Group pages by category
    for page in pages {
        if let PageType::Item = page.page_type {
            if let Some(category) = &page.category {
                categories.entry(category.clone()).or_default().push(page);
            }
        }
    }
    
    let mut site_categories = Vec::new();
    
    // Convert to Category structures
    for (category_name, item_pages) in categories {
        let mut items = Vec::new();
        
        for page in item_pages {
            let item = ContentItem {
                category: category_name.clone(),
                item_name: page.slug.clone(),
                dir_name: page.slug.clone(),
                url: page.slug.clone(),
                file_path: std::path::PathBuf::new(),
                title: page.title.clone(),
                date: None, // Could extract from updated_at if needed
                author: None,
                description: None, // Could be extracted from content
                html_content: page.content.clone(),
                attachments: Vec::new(),
                tags: Vec::new(),
            };
            items.push(item);
        }
        
        let category = Category {
            name: category_name.clone(),
            url: category_name.clone(),
            index_path: std::path::PathBuf::new(),
            items,
            description: None,
        };
        
        site_categories.push(category);
    }
    
    SiteContent {
        categories: site_categories,
    }
}

/// Configure routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Configure admin routes first to avoid being matched by wildcard route
    admin::configure_admin_routes(cfg);

    // Configure analytics API routes
    cfg
        .route("/health", web::get().to(health_check))
        .route("/api/recompile", web::post().to(recompile))
        // Analytics API endpoints
        .route("/api/analytics/daily-stats", web::get().to(analytics::get_daily_stats))
        .route("/api/analytics/source-stats", web::get().to(analytics::get_source_stats))
        .route("/api/analytics/popular-pages", web::get().to(analytics::get_popular_pages))
        .route("/api/analytics/total-stats", web::get().to(analytics::get_total_stats))
        .route("/api/analytics/search-keywords", web::get().to(analytics::get_search_keywords))
        .route("/api/analytics/record-visit/{page_slug:.*}", web::post().to(analytics::record_visit))
        // Comments API endpoints
        .route("/api/comments/{slug}", web::get().to(get_comments))
        .route("/api/comments", web::post().to(add_comment))
        // Recommender API endpoints
        .route("/api/recommendations/{url:.*}", web::get().to(get_recommendations))
        .route("/api/popular-content", web::get().to(get_popular_content))
        .route("/api/latest-content", web::get().to(get_latest_content))
        // Search API endpoint
        .route("/api/search", web::get().to(search_content))
        // Main site routes
        .route("/", web::get().to(index))
        .route("/search", web::get().to(search_page))
        .route("/attachment/{filename:.*}", web::get().to(serve_attachment))
        .route("/{path:.*}", web::get().to(serve_content));
}