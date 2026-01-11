use chrono::{DateTime, NaiveDateTime};
use pulldown_cmark::{html, Event, Parser, Tag};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "time")]
    #[serde(default)]
    pub time: Option<String>, // Support both 'date' and 'time'
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMarkdown {
    pub frontmatter: Frontmatter,
    pub html_content: String,
    pub raw_content: String,
}

impl ParsedMarkdown {
    pub fn get_title(&self) -> String {
        // Try to get title from frontmatter
        if let Some(title) = &self.frontmatter.title {
            return title.clone();
        }

        // Try to extract title from the first H1
        let parser = Parser::new(&self.raw_content);
        for event in parser {
            if let Event::Start(Tag::Heading {
                level: pulldown_cmark::HeadingLevel::H1,
                ..
            }) = event
            {
                return String::new(); // Will be filled by the next event
            }
            if let Event::Text(text) = event {
                return text.to_string();
            }
        }

        // Fallback to the first line
        self.raw_content
            .lines()
            .next()
            .unwrap_or("Untitled")
            .to_string()
    }

    pub fn get_date(&self) -> Option<NaiveDateTime> {
        // Try 'time' first, then 'date'
        let date_str = self
            .frontmatter
            .time
            .as_ref()
            .or(self.frontmatter.date.as_ref())?;
        DateTime::parse_from_rfc3339(date_str)
            .ok()
            .map(|dt| dt.naive_utc())
            .or_else(|| {
                // Try other formats
                NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d").ok()
                // Can add more formats here
            })
    }
}

pub fn parse_markdown(content: &str) -> Result<ParsedMarkdown, Box<dyn std::error::Error>> {
    // Parse frontmatter
    let (frontmatter, markdown_content) = parse_frontmatter(content)?;

    // Convert markdown to HTML
    let parser = Parser::new(markdown_content);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(ParsedMarkdown {
        frontmatter,
        html_content: html_output,
        raw_content: markdown_content.to_string(),
    })
}

fn parse_frontmatter(content: &str) -> Result<(Frontmatter, &str), Box<dyn std::error::Error>> {
    if !content.starts_with("---") {
        // No frontmatter
        return Ok((
            Frontmatter {
                title: None,
                date: None,
                author: None,
                tags: None,
                description: None,
                time: None,
                extra: HashMap::new(),
            },
            content,
        ));
    }

    // Find the end of frontmatter
    let second_separator = content[3..]
        .find("---")
        .ok_or("No closing frontmatter separator")?;

    let frontmatter_str = &content[3..3 + second_separator];
    let markdown_content = &content[3 + second_separator + 3..].trim_start();

    // Parse YAML frontmatter
    let frontmatter: Frontmatter = serde_yaml::from_str(frontmatter_str)?;

    Ok((frontmatter, markdown_content))
}

// Handle image links with relative paths
pub fn process_image_links(html: &str, base_path: &str) -> String {
    // This is a simple implementation - for more complex scenarios, use a proper HTML parser
    let result = html.replace(
        &format!("./attachment/"),
        &format!("{}/attachment/", base_path),
    );
    result
}

/// Replace attachment links in HTML content with their new names
/// Original format: ./attachment/filename.ext or attachment/filename.ext
/// New format: attachment/new_filename.ext
pub fn replace_attachment_links(html: &str, attachment_map: &[(String, String)]) -> String {
    let mut result = html.to_string();

    // Sort by length descending to avoid partial replacements
    // e.g., replace "image.png" before replacing "image_small.png"
    let mut sorted_attachments: Vec<_> = attachment_map.iter().collect();
    sorted_attachments.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (original_name, new_name) in sorted_attachments {
        // Match patterns: ./attachment/filename or attachment/filename
        let patterns = [
            format!("./attachment/{}", original_name),
            format!("attachment/{}", original_name),
        ];

        for pattern in &patterns {
            result = result.replace(pattern, &format!("attachment/{}", new_name));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown() {
        let content = r#"---
title: Test Post
date: 2026-01-08
author: Test Author
---

# Hello World

This is a test post."#;

        let parsed = parse_markdown(content).unwrap();
        assert_eq!(parsed.frontmatter.title, Some("Test Post".to_string()));
        assert_eq!(parsed.frontmatter.author, Some("Test Author".to_string()));
        assert!(parsed.html_content.contains("<h1>Hello World</h1>"));
    }

    #[test]
    fn test_parse_without_frontmatter() {
        let content = "# Hello World\n\nThis is a test post.";
        let parsed = parse_markdown(content).unwrap();
        assert!(parsed.frontmatter.title.is_none());
        assert!(parsed.html_content.contains("<h1>Hello World</h1>"));
    }
}
