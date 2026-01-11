use crate::config::Config;
use crate::scanner::{Attachment, Scanner, SiteContent};
use crate::storage::{Page, PageType, StoredAttachment, StorageDB};
use crate::templates::TemplateRenderer;
use crate::markdown::replace_attachment_links;
use std::path::PathBuf;

pub struct Compiler {
    config: Config,
    scanner: Scanner,
    renderer: TemplateRenderer,
    storage: StorageDB,
}

impl Compiler {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let scanner = Scanner::new(config.paths.content_dir.clone());
        let renderer = TemplateRenderer::new(&config)?;

        // Initialize storage database
        let storage_path = config.paths.storage_database_path.to_string_lossy().to_string();
        let storage = StorageDB::new(&storage_path)?;

        Ok(Self {
            config,
            scanner,
            renderer,
            storage,
        })
    }

    /// Compile the entire site - stores everything in database
    pub fn compile(&mut self) -> Result<CompileResult, Box<dyn std::error::Error>> {
        log::info!("Starting compilation...");

        // Scan content
        let site_content = self.scanner.scan()?;
        log::info!("Found {} categories", site_content.categories.len());

        // Collect pages and attachments for storage
        let mut pages_to_save: Vec<Page> = Vec::new();
        let mut attachments_to_save: Vec<StoredAttachment> = Vec::new();
        let mut total_attachments_count = 0;

        // Render index page
        let index_html = self.renderer.render_index(&site_content)?;

        // Save index page to storage
        let now = chrono::Utc::now().to_rfc3339();
        pages_to_save.push(Page {
            id: "index".to_string(),
            slug: "index".to_string(),
            page_type: PageType::Index,
            title: self.config.site.title.clone(),
            content: index_html,
            category: None,
            updated_at: now.clone(),
        });
        log::info!("Compiled index page");

        // Render category pages
        for category in &site_content.categories {
            let category_html = self.renderer.render_category(category)?;

            // Save category page to storage
            pages_to_save.push(Page {
                id: format!("category-{}", category.url),
                slug: category.url.clone(),
                page_type: PageType::Category,
                title: category.name.clone(),
                content: category_html,
                category: None,
                updated_at: now.clone(),
            });
            log::info!("Compiled category: {}", category.name);

            // Render item pages and collect attachments
            for item in &category.items {
                // Build attachment map for this item
                let attachment_map: Vec<(String, String)> = item.attachments.iter()
                    .map(|a| (a.original_name.clone(), a.new_name.clone()))
                    .collect();

                // Render item HTML and replace attachment links
                let raw_item_html = self.renderer.render_item(item)?;
                let item_html = replace_attachment_links(&raw_item_html, &attachment_map);

                // Save item page to storage
                pages_to_save.push(Page {
                    id: format!("item-{}", item.url),
                    slug: item.url.clone(),
                    page_type: PageType::Item,
                    title: item.title.clone(),
                    content: item_html,
                    category: Some(item.category.clone()),
                    updated_at: now.clone(),
                });

                // Collect attachments for this item
                for attachment in &item.attachments {
                    let stored_attachment = StoredAttachment {
                        id: format!("attachment-{}", attachment.new_name),
                        slug: item.url.clone(),
                        filename: attachment.new_name.clone(),
                        original_name: attachment.original_name.clone(),
                        mime_type: attachment.mime_type.clone(),
                        file_data: attachment.file_data.clone(),
                        file_size: attachment.file_size,
                        updated_at: now.clone(),
                    };
                    attachments_to_save.push(stored_attachment);
                    total_attachments_count += 1;
                }
                log::info!("Compiled item: {} ({} attachments)", item.title, item.attachments.len());
            }
        }

        // Save all pages to storage database
        self.storage.save_pages_batch(&pages_to_save)?;
        log::info!("Saved {} pages to storage database", pages_to_save.len());

        // Save all attachments to storage database
        if !attachments_to_save.is_empty() {
            self.storage.save_attachments_batch(&attachments_to_save)?;
            log::info!("Saved {} attachments to storage database", attachments_to_save.len());
        }

        // Update compile time
        self.storage.update_compile_time()?;

        log::info!("Compilation complete. {} pages, {} attachments.", pages_to_save.len(), total_attachments_count);

        // Return result with empty generated_files since everything is in database
        Ok(CompileResult {
            success: true,
            generated_files: Vec::new(),
            total_categories: site_content.categories.len(),
            total_items: site_content.categories.iter().map(|c| c.items.len()).sum(),
            total_attachments: total_attachments_count,
        })
    }

    fn generate_sitemap(&self, site_content: &SiteContent, generated_files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        let mut sitemap = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        sitemap.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        // Add homepage
        sitemap.push_str(&format!(
            "  <url><loc>{}/</loc><changefreq>daily</changefreq></url>\n",
            self.config.site.url
        ));

        // Add category pages
        for category in &site_content.categories {
            sitemap.push_str(&format!(
                "  <url><loc>{}/{}</loc><changefreq>weekly</changefreq></url>\n",
                self.config.site.url, category.url
            ));

            // Add item pages
            for item in &category.items {
                sitemap.push_str(&format!(
                    "  <url><loc>{}/{}</loc><changefreq>monthly</changefreq></url>\n",
                    self.config.site.url, item.url
                ));
            }
        }

        sitemap.push_str("</urlset>");

        let sitemap_path = self.config.paths.generated_dir.join("sitemap.xml");
        std::fs::write(&sitemap_path, sitemap)?;
        generated_files.push(sitemap_path);
        log::info!("Generated sitemap.xml");

        Ok(())
    }

    fn generate_robots_txt(&self, generated_files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        let robots_txt = format!(
            r#"User-agent: *
Allow: /
Sitemap: {}/sitemap.xml
"#,
            self.config.site.url
        );

        let robots_path = self.config.paths.generated_dir.join("robots.txt");
        std::fs::write(&robots_path, robots_txt)?;
        generated_files.push(robots_path);
        log::info!("Generated robots.txt");

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub success: bool,
    pub generated_files: Vec<PathBuf>,
    pub total_categories: usize,
    pub total_items: usize,
    pub total_attachments: usize,
}