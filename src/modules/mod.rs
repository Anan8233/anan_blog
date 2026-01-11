//! LF Blog 模块组织结构
//!
//! 核心配置和工具
pub mod config;
pub mod cli;

// 数据库相关模块
pub mod database {
    pub mod storage;
    pub mod comments;
    pub mod analytics;
}

// 内容处理相关模块
pub mod content {
    pub mod scanner;
    pub mod compiler;
    pub mod markdown;
    pub mod templates;
    pub mod theme;
}

// Web 相关模块
pub mod web {
    pub mod admin;
    pub mod routes;
    pub mod recommender;
}