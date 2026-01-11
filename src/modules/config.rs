use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub paths: PathConfig,
    pub site: SiteConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub admin_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub content_dir: PathBuf,
    pub generated_dir: PathBuf,
    pub static_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub database_path: PathBuf,         // 评论数据库路径
    pub storage_database_path: PathBuf, // 统一存储数据库路径
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub title: String,
    pub description: String,
    pub url: String,
    pub author: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                admin_password: "admin".to_string(),
            },
            paths: PathConfig {
                content_dir: PathBuf::from("content"),
                generated_dir: PathBuf::from("generated"),
                static_dir: PathBuf::from("static"),
                templates_dir: PathBuf::from("templates"),
                database_path: PathBuf::from("comments.db"),
                storage_database_path: PathBuf::from("storage.db"),
            },
            site: SiteConfig {
                title: "My Collections".to_string(),
                description: "A blog about my collections".to_string(),
                url: "http://localhost:8080".to_string(),
                author: "xiaolinfeng".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load from config file, fall back to default
        let config_path = PathBuf::from("lf_blog.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write("lf_blog.toml", content)?;
        Ok(())
    }
}
