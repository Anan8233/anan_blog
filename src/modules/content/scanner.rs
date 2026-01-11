use crate::markdown::parse_markdown;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Attachment file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub original_name: String, // Original filename
    pub new_name: String,      // Generated unique filename
    pub file_type: String,     // File extension (e.g., "jpg", "png")
    pub path: String,          // Relative path in generated folder
    pub file_data: Vec<u8>,    // File binary content
    pub file_size: usize,      // File size in bytes
    pub mime_type: String,     // MIME type
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub category: String,     // e.g., "grape"
    pub item_name: String,    // e.g., "tizi"
    pub dir_name: String,     // e.g., "提子" (actual directory name)
    pub url: String,          // e.g., "grape-tizi"
    pub file_path: PathBuf,   // e.g., "content/grape/tizi/tizi.md"
    pub title: String,        // Extracted title
    pub date: Option<String>, // Formatted date string
    pub author: Option<String>,
    pub description: Option<String>,
    pub html_content: String,
    pub attachments: Vec<Attachment>, // List of attachments
    pub tags: Vec<String>,            // Tags from frontmatter
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,        // e.g., "grape"
    pub url: String,         // e.g., "grape"
    pub index_path: PathBuf, // e.g., "content/grape/index.md"
    pub items: Vec<ContentItem>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteContent {
    pub categories: Vec<Category>,
}

pub struct Scanner {
    content_dir: PathBuf,
}

impl Scanner {
    pub fn new(content_dir: PathBuf) -> Self {
        Self { content_dir }
    }

    /// Scan the content directory and build the site structure
    pub fn scan(&self) -> Result<SiteContent, Box<dyn std::error::Error>> {
        let mut categories = Vec::new();

        // Check if content directory exists
        if !self.content_dir.exists() {
            log::warn!("Content directory does not exist: {:?}", self.content_dir);
            return Ok(SiteContent { categories });
        }

        // Iterate over subdirectories in content directory
        for entry in std::fs::read_dir(&self.content_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }

            // Get category name from directory name
            let category_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid category name")?;

            // Skip hidden directories
            if category_name.starts_with('.') {
                continue;
            }

            // Scan this category
            if let Some(category) = self.scan_category(&path, category_name)? {
                categories.push(category);
            }
        }

        // Sort categories alphabetically
        categories.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(SiteContent { categories })
    }

    fn scan_category(
        &self,
        category_path: &Path,
        category_name: &str,
    ) -> Result<Option<Category>, Box<dyn std::error::Error>> {
        let index_path = category_path.join("index.md");
        let mut items = Vec::new();
        let mut description = None;

        // Parse index.md if exists
        if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            if let Ok(parsed) = parse_markdown(&content) {
                description = parsed.frontmatter.description.clone();
            }
        }

        // Scan subdirectories for items
        for entry in std::fs::read_dir(category_path)? {
            let entry = entry?;
            let path = entry.path();

            // Skip files (we only care about subdirectories)
            if !path.is_dir() {
                continue;
            }

            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid item name")?;

            // Skip hidden directories and index files
            if dir_name.starts_with('.') || dir_name == "attachment" {
                continue;
            }

            // Try to find any markdown file in the directory
            for entry in std::fs::read_dir(&path)? {
                let entry = entry?;
                let md_path = entry.path();

                // Only process .md files
                if md_path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }

                // Get item name from filename (without .md extension)
                let item_name = md_path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .ok_or("Invalid item name")?;

                if let Some(item) =
                    self.scan_item(&md_path, category_name, dir_name, item_name, &path)?
                {
                    items.push(item);
                }
            }
        }

        // If no items and no index, skip this category
        if items.is_empty() && !index_path.exists() {
            return Ok(None);
        }

        // Sort items by date (newest first) or alphabetically
        items.sort_by(|a, b| match (&a.date, &b.date) {
            (Some(da), Some(db)) => db.cmp(da),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.item_name.cmp(&b.item_name),
        });

        Ok(Some(Category {
            name: category_name.to_string(),
            url: category_name.to_string(),
            index_path,
            items,
            description,
        }))
    }

    fn scan_item(
        &self,
        md_path: &Path,
        category_name: &str,
        dir_name: &str,
        item_name: &str,
        item_dir: &Path,
    ) -> Result<Option<ContentItem>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(md_path)?;
        let parsed = parse_markdown(&content)?;

        // Scan attachments
        let attachments = self.scan_attachments(item_dir)?;

        // Generate URL: category-itemname (use item_name for URL)
        let url = format!("{}-{}", category_name, item_name);

        // Extract title
        let title = parsed.get_title();

        // Format date if available
        let date = parsed.get_date().map(|d| d.format("%Y-%m-%d").to_string());

        Ok(Some(ContentItem {
            category: category_name.to_string(),
            item_name: item_name.to_string(),
            dir_name: dir_name.to_string(), // Store the actual directory name
            url,
            file_path: md_path.to_path_buf(),
            title,
            date,
            author: parsed.frontmatter.author.clone(),
            description: parsed.frontmatter.description.clone(),
            html_content: parsed.html_content,
            attachments,
            tags: parsed.frontmatter.tags.clone().unwrap_or_default(),
        }))
    }

    /// Scan attachments in the item's attachment directory
    fn scan_attachments(
        &self,
        item_dir: &Path,
    ) -> Result<Vec<Attachment>, Box<dyn std::error::Error>> {
        let attachment_dir = item_dir.join("attachment");

        if !attachment_dir.exists() || !attachment_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut attachments = Vec::new();
        let mut counter: usize = 0;

        // Track used names to avoid duplicates
        let mut used_names = std::collections::HashSet::new();

        for entry in std::fs::read_dir(&attachment_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Invalid file name")?;

            // Skip hidden files
            if file_name.starts_with('.') {
                continue;
            }

            // Get file extension
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();

            // Generate unique name: {item_name}_{counter}.{ext}
            let item_name = item_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("item");

            counter += 1;
            let mut new_name = format!("{}_{}.{}", item_name, counter, extension);

            // Ensure uniqueness
            while used_names.contains(&new_name) {
                counter += 1;
                new_name = format!("{}_{}.{}", item_name, counter, extension);
            }
            used_names.insert(new_name.clone());

            // Read file content
            let file_data = std::fs::read(&path)?;
            let file_size = file_data.len();
            let mime_type = get_mime_type(&extension);

            attachments.push(Attachment {
                original_name: file_name.to_string(),
                new_name: new_name.clone(),
                file_type: extension,
                path: format!("attachment/{}", new_name),
                file_data,
                file_size,
                mime_type,
            });
        }

        Ok(attachments)
    }
}

/// Get MIME type based on file extension
fn get_mime_type(extension: &str) -> String {
    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "png" => "image/png".to_string(),
        "gif" => "image/gif".to_string(),
        "svg" => "image/svg+xml".to_string(),
        "webp" => "image/webp".to_string(),
        "pdf" => "application/pdf".to_string(),
        "zip" => "application/zip".to_string(),
        "mp4" => "video/mp4".to_string(),
        "mp3" => "audio/mpeg".to_string(),
        "txt" => "text/plain".to_string(),
        "css" => "text/css".to_string(),
        "js" => "application/javascript".to_string(),
        "html" => "text/html".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_generation() {
        assert_eq!(format!("{}-{}", "grape", "tizi"), "grape-tizi");
    }
}