use clap::{Parser, Subcommand, Command};
use std::path::PathBuf;
use std::fs;
use chrono::Utc;

#[derive(Parser, Debug)]
#[command(name = "lfb")]
#[command(author = "xiaolinfeng")]
#[command(version = "0.1.0")]
#[command(about = "LF Blog CLI tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(name = "client")]
    #[command(about = "Local client commands")]
    Client {
        #[command(subcommand)]
        action: ClientActions,

        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },

    #[command(name = "server")]
    #[command(about = "Start the blog server")]
    Server(ServerArgs),
}

#[derive(Subcommand, Debug)]
pub enum ClientActions {
    #[command(name = "new_list")]
    #[command(about = "Create a new category/list")]
    NewList {
        #[arg(help = "Name of the new category")]
        name: String,
    },

    #[command(name = "new_page")]
    #[command(about = "Create a new article page")]
    NewPage {
        #[arg(help = "Title of the new article")]
        title: String,
    },
}

#[derive(Parser, Debug)]
pub struct ServerArgs {
    #[arg(short, long, default_value = "lf_blog.toml")]
    pub config: PathBuf,
}

pub fn run() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Client { action, dir } => {
            let base_dir = match fs::canonicalize(dir) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Error: Could not resolve directory '{}': {}", dir.display(), e);
                    std::process::exit(1);
                }
            };

            match action {
                ClientActions::NewList { name } => {
                    if let Err(e) = create_category(&base_dir, name) {
                        eprintln!("Error creating category: {}", e);
                        std::process::exit(1);
                    }
                }
                ClientActions::NewPage { title } => {
                    if let Err(e) = create_page(&base_dir, title) {
                        eprintln!("Error creating page: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Server(args) => {
            println!("Starting server with config: {}", args.config.display());
            println!("Note: Server mode is typically run via 'cargo run' or the binary directly.");
            println!("To start the server, run: lfg_server --config {}", args.config.display());
        }
    }
}

fn create_category(base_dir: &PathBuf, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let slug = to_slug(name);
    let category_dir = base_dir.join(&slug);

    if category_dir.exists() {
        return Err(format!("Category '{}' already exists at '{}'", name, category_dir.display()).into());
    }

    fs::create_dir_all(&category_dir)?;

    let index_content = format!(
        r#"---
title: {}
date: {}
---

# {}

欢迎来到分类页面。

你可以在这里添加分类的介绍内容。
"#,
        name,
        Utc::now().to_rfc3339(),
        name
    );

    let index_path = category_dir.join("index.md");
    fs::write(&index_path, &index_content)?;

    println!("✓ Category '{}' created at: {}", name, category_dir.display());
    println!("  - Created: {}", index_path.display());
    println!("  - Slug: {}", slug);

    Ok(())
}

fn create_page(base_dir: &PathBuf, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let slug = to_slug(title);
    let page_dir = base_dir.join(&slug);

    if page_dir.exists() {
        return Err(format!("Page '{}' already exists at '{}'", title, page_dir.display()).into());
    }

    fs::create_dir_all(&page_dir)?;
    fs::create_dir_all(&page_dir.join("attachment"))?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{}.md", slug);

    let page_content = format!(
        r#"---
title: {}
date: {}
author: 
tags: []
description: 
---

# {}

在这里开始写你的文章内容...

## 标题2

- 列表项1
- 列表项2

```rust
fn main() {{
    println!("Hello, World!");
}}
```
"#,
        title,
        Utc::now().to_rfc3339(),
        title
    );

    let page_path = page_dir.join(&filename);
    fs::write(&page_path, &page_content)?;

    println!("✓ Page '{}' created at: {}", title, page_dir.display());
    println!("  - Created: {}", page_path.display());
    println!("  - Attachment dir: {}/attachment", page_dir.display());
    println!("  - Slug: {}", slug);

    Ok(())
}

fn to_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .chars()
        .collect::<String>()
}
