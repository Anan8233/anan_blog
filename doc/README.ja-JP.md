# LF Blog

LF Blog は、Markdown、ファイル管理、コメント、統計分析機能を備えた Rust で書かれた静的ブログシステムです。

## 機能

- **静的ブログ生成**: Markdown コンテンツを静的 HTML ページにコンパイル
- **ファイル管理**: Web インターフェースから添付ファイルをアップロード・管理
- **コメントシステム**: SQLite ベースの組み込みコメント機能
- **统计分析**: ページビューと訪問者統計のトラッキング
- **CLI ツール**: コンテンツ管理用のコマンドラインインターフェース
- **テンプレートシステム**: Tera テンプレートエンジンを使用した柔軟なテーマシステム
- **リアルタイムコンパイル**: ファイル変更時に自動的にコンテンツを再コンパイル

## クイックスタート

### 必要環境

- Rust 1.70 以降
- Cargo パッケージマネージャー

### インストール

```bash
# リポジトリのクローン
git clone <repository-url>
cd lf_blog

# プロジェクトのビルド
cargo build --release
```

### 設定

`lf_blog.toml` 設定ファイルを作成：

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
title = "マイコレクション"
description = "私のコレクションに関するブログ"
url = "http://localhost:8080"
author = "xiaolinfeng"
```

### サーバーの実行

```bash
# Web サーバーモードで実行（デフォルト）
cargo run

# または CLI モードを使用
cargo run client --help
cargo run server --help
```

サーバーはデフォルトで `http://127.0.0.1:8080` で起動します。

### コンテンツ構造

`content` ディレクトリにコンテンツを整理：

```
content/
├── カテゴリー1/
│   ├── 記事1.md
│   ├── 記事2.md
│   └── 添付ファイル/
│       └── 画像1.jpg
├── カテゴリー2/
│   └── 記事3.md
```

### CLI コマンド

```bash
# 新しいカテゴリーを作成
cargo run client new_list "カテゴリー名" --dir /path/to/content

# 新しいページを作成
cargo run client new_page "ページタイトル" --dir /path/to/content
```

## プロジェクト構造

```
lf_blog/
├── src/
│   ├── main.rs                    # エントリーポイント
│   └── modules/
│       ├── config.rs              # 設定管理
│       ├── cli/                   # CLI コマンド
│       ├── database/              # データベース操作
│       │   ├── storage.rs         # 統一ストレージ
│       │   ├── comments.rs        # コメントシステム
│       │   └── analytics.rs       # 統計分析
│       ├── content/               # コンテンツ処理
│       │   ├── scanner.rs         # ファイルスキャン
│       │   ├── compiler.rs        # サイトコンパイル
│       │   ├── markdown.rs        # Markdown 解析
│       │   ├── templates.rs       # テンプレートレンダリング
│       │   └── theme.rs           # テーマ管理
│       └── web/                   # Web ハンドラー
│           ├── routes.rs          # HTTP ルーティング
│           ├── admin.rs           # 管理画面
│           └── recommender.rs     # コンテンツ推奨
├── content/                       # ブログコンテンツ
├── static/                        # 静的アセット
├── templates/                     # HTML テンプレート
├── generated/                     # コンパイル出力
├── doc/                           # ドキュメント
└── Cargo.toml                     # Rust プロジェクトファイル
```

## 使用技術

- **Web フレームワーク**: [Actix-web](https://actix.rs/)
- **テンプレートエンジン**: [Tera](https://tera.netlify.app/)
- **Markdown パーサー**: [pulldown-cmark](https://github.com/google/pulldown-cmark)
- **データベース**: [SQLite](https://www.sqlite.org/) via rusqlite
- **ビルドツール**: [Cargo](https://doc.rust-lang.org/cargo/)

## ライセンス

MIT License
