# LF Blog

LF Blog 是一个使用 Rust 编写的静态博客系统，支持 Markdown、文件管理、评论和统计分析等功能。

## 功能特性

- **静态博客生成**: 将 Markdown 内容编译为静态 HTML 页面
- **文件管理**: 通过 Web 界面上传和管理附件
- **评论系统**: 内置基于 SQLite 的评论功能
- **统计分析**: 跟踪页面浏览量和访客统计
- **命令行工具**: 用于管理内容的 CLI 界面
- **模板系统**: 使用 Tera 模板引擎的灵活主题系统
- **实时编译**: 文件更改时自动重新编译内容

## 快速开始

### 环境要求

- Rust 1.70 或更高版本
- Cargo 包管理器

### 安装

```bash
# 克隆仓库
git clone <repository-url>
cd lf_blog

# 构建项目
cargo build --release
```

### 配置

创建 `lf_blog.toml` 配置文件：

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
title = "我的收藏"
description = "关于我收藏品的博客"
url = "http://localhost:8080"
author = "xiaolinfeng"
```

### 运行服务器

```bash
# 以 Web 服务器模式运行（默认）
cargo run

# 或使用 CLI 模式
cargo run client --help
cargo run server --help
```

服务器默认在 `http://127.0.0.1:8080` 启动。

### 内容结构

在 `content` 目录中组织内容：

```
content/
├── 分类1/
│   ├── 文章1.md
│   ├── 文章2.md
│   └── 附件/
│       └── 图片1.jpg
├── 分类2/
│   └── 文章3.md
```

### CLI 命令

```bash
# 创建新分类
cargo run client new_list "分类名称" --dir /path/to/content

# 创建新页面
cargo run client new_page "页面标题" --dir /path/to/content
```

## 项目结构

```
lf_blog/
├── src/
│   ├── main.rs                    # 入口点
│   └── modules/
│       ├── config.rs              # 配置管理
│       ├── cli/                   # CLI 命令
│       ├── database/              # 数据库操作
│       │   ├── storage.rs         # 统一存储
│       │   ├── comments.rs        # 评论系统
│       │   └── analytics.rs       # 统计分析
│       ├── content/               # 内容处理
│       │   ├── scanner.rs         # 文件扫描
│       │   ├── compiler.rs        # 站点编译
│       │   ├── markdown.rs        # Markdown 解析
│       │   ├── templates.rs       # 模板渲染
│       │   └── theme.rs           # 主题管理
│       └── web/                   # Web 处理器
│           ├── routes.rs          # HTTP 路由
│           ├── admin.rs           # 管理界面
│           └── recommender.rs     # 内容推荐
├── content/                       # 博客内容
├── static/                        # 静态资源
├── templates/                     # HTML 模板
├── generated/                     # 编译输出
├── doc/                           # 文档
└── Cargo.toml                     # Rust 项目文件
```

## 技术栈

- **Web 框架**: [Actix-web](https://actix.rs/)
- **模板引擎**: [Tera](https://tera.netlify.app/)
- **Markdown 解析器**: [pulldown-cmark](https://github.com/google/pulldown-cmark)
- **数据库**: [SQLite](https://www.sqlite.org/) via rusqlite
- **构建工具**: [Cargo](https://doc.rust-lang.org/cargo/)

## 许可证

MIT License
