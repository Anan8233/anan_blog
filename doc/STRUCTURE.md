# LF Blog 项目结构优化

这个项目已经进行了模块化重组，以改善代码组织和维护性。

## 新的目录结构

```
lf_blog/
├── src/
│   ├── main.rs                 # 程序入口点
│   └── modules/               # 主要模块目录
│       ├── mod.rs              # 模块声明文件
│       ├── config.rs           # 全局配置管理
│       ├── 
│       ├── database/           # 数据库相关模块
│       │   ├── mod.rs
│       │   ├── storage.rs      # 中央存储数据库
│       │   ├── comments.rs     # 评论系统
│       │   └── analytics.rs    # 访问统计分析
│       │
│       ├── content/            # 内容处理相关模块
│       │   ├── mod.rs
│       │   ├── scanner.rs      # 文件扫描器
│       │   ├── compiler.rs     # 内容编译器
│       │   ├── markdown.rs     # Markdown 解析器
│       │   ├── templates.rs    # 模板渲染引擎
│       │   └── theme.rs        # 主题管理
│       │
│       └── web/               # Web 相关模块
│           ├── mod.rs
│           ├── admin.rs       # 管理员接口
│           ├── routes.rs      # HTTP 路由定义
│           └── recommender.rs # 内容推荐系统
│
├── static/                     # 静态资源文件
├── templates/                  # HTML 模板文件
└── content/                    # Markdown 内容源文件
```

## 模块职责说明

### Database 模块 (`src/modules/database/`)
- **storage.rs**: 处理所有内容数据在 SQLite 数据库中的存储和检索
- **comments.rs**: 管理文章评论系统
- **analytics.rs**: 处理访问统计、来源分析等数据收集

### Content 模块 (`src/modules/content/`)
- **scanner.rs**: 扫描文件系统中的内容文件和附件
- **compiler.rs**: 将 Markdown 内容编译为 HTML 页面
- **markdown.rs**: Markdown 解析和链接处理
- **templates.rs**: HTML 模板渲染引擎
- **theme.rs**: CSS 主题和样式管理

### Web 模块 (`src/modules/web/`)
- **admin.rs**: 管理员界面、内容管理、上传功能
- **routes.rs**: 所有 HTTP 路由处理器
- **recommender.rs**: 基于内容相似度的推荐系统

### 核心模块
- **config.rs**: 全局配置管理，包括服务器设置、路径配置等

## 主要改进

1. **更清晰的职责分离**: 每个模块具有明确的职责范围
2. **更好的文件组织**: 相关功能被组织在同一个目录中
3. **易于维护**: 新功能可以更容易地添加到合适的模块中
4. **更好的可读性**: 模块结构反映了系统的功能架构

## 构建和使用

项目保持完全向后兼容，原有的构建命令和功能都可以正常工作:

```bash
cargo run          # 运行开发服务器
cargo build       # 构建项目
cargo test        # 运行测试
```

## 功能完整性

✅ 所有原有功能保持完整
✅ 管理员界面和内容管理
✅ Markdown 编译和静态页面生成  
✅ 评论系统
✅ 访问统计分析
✅ 主题和夜间模式支持
✅ 内容推荐系统

这个新的结构让项目更加易，便于未来的功能扩展和维护。