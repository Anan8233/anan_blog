# LF Blog

A static blog system written in Rust, featuring Markdown support, file management, comments, and analytics.

## Other Languages

- [中文 (Chinese)](doc/README.zh-CN.md)
- [日本語 (Japanese)](doc/README.ja-JP.md)

## Features

- **Static Blog Generation**: Compile Markdown content into static HTML pages
- **File Management**: Upload and manage attachments via web interface
- **Comment System**: Built-in SQLite-based comment functionality
- **Analytics**: Track page views and visitor statistics
- **CLI Tools**: Command-line interface for managing content
- **Template System**: Flexible theming with Tera template engine
- **Real-time Compilation**: Auto-compile content when files change

## Quick Start

### Prerequisites

- Rust 1.70 or later
- Cargo package manager

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd lf_blog

# Build the project
cargo build --release
```

### Configuration

Create a `lf_blog.toml` configuration file:

```toml
[server]
host = "127.0.0.1"
port = 8080
admin_password = "admin"

[paths]
content_dir = "content"
generated_dir = "generated"
static_dir = "static"
templates_dir = "templates"
database_path = "comments.db"
storage_database_path = "storage.db"

[site]
title = "My Collections"
description = "A blog about my collections"
url = "http://localhost:8080"
author = "xiaolinfeng"
```

### Running the Server

```bash
# Run in web server mode (default)
cargo run

# Or use CLI mode
cargo run client --help
cargo run server --help
```

The server will start at `http://127.0.0.1:8080` by default.

### Content Structure

Organize your content in the `content` directory:

```
content/
├── category1/
│   ├── item1.md
│   ├── item2.md
│   └── attachments/
│       └── image1.jpg
├── category2/
│   └── item3.md
```

### CLI Commands

```bash
# Create a new category
cargo run client new_list "Category Name" --dir /path/to/content

# Create a new page
cargo run client new_page "Page Title" --dir /path/to/content
```

## Project Structure

```
lf_blog/
├── src/
│   ├── main.rs                    # Entry point
│   └── modules/
│       ├── config.rs              # Configuration management
│       ├── cli/                   # CLI commands
│       ├── database/              # Database operations
│       │   ├── storage.rs         # Unified storage
│       │   ├── comments.rs        # Comment system
│       │   └── analytics.rs       # Analytics tracking
│       ├── content/               # Content processing
│       │   ├── scanner.rs         # File scanning
│       │   ├── compiler.rs        # Site compilation
│       │   ├── markdown.rs        # Markdown parsing
│       │   ├── templates.rs       # Template rendering
│       │   └── theme.rs           # Theme management
│       └── web/                   # Web handlers
│           ├── routes.rs          # HTTP routes
│           ├── admin.rs           # Admin interface
│           └── recommender.rs     # Content recommendations
├── content/                       # Your blog content
├── static/                        # Static assets
├── templates/                     # HTML templates
├── generated/                     # Compiled output
├── doc/                           # Documentation
└── Cargo.toml                     # Rust project file
```

## Technologies

- **Web Framework**: [Actix-web](https://actix.rs/)
- **Template Engine**: [Tera](https://tera.netlify.app/)
- **Markdown Parser**: [pulldown-cmark](https://github.com/google/pulldown-cmark)
- **Database**: [SQLite](https://www.sqlite.org/) via rusqlite
- **Build Tool**: [Cargo](https://doc.rust-lang.org/cargo/)

## License

MIT License
