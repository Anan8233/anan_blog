use crate::scanner::{Category, ContentItem, SiteContent};
use serde::Serialize;
use std::collections::HashMap;

/// 推荐内容结构
#[derive(Debug, Clone, Serialize)]
pub struct RecommendedItem {
    pub url: String,
    pub title: String,
    pub category: String,
    pub category_name: String,
    pub description: Option<String>,
    pub score: f64,
}

/// 推荐系统
pub struct Recommender {
    site_content: SiteContent,
    /// 关键词到文章索引的映射
    keyword_index: HashMap<String, Vec<usize>>,
}

impl Recommender {
    /// 创建新的推荐系统
    pub fn new(site_content: SiteContent) -> Self {
        let mut keyword_index = HashMap::new();

        // 构建关键词索引
        for (index, item) in site_content.all_items().into_iter().enumerate() {
            let keywords = Self::extract_keywords(item);
            for keyword in keywords {
                keyword_index
                    .entry(keyword.to_lowercase())
                    .or_insert_with(Vec::new)
                    .push(index);
            }
        }

        Self {
            site_content,
            keyword_index,
        }
    }

    /// 提取文章的关键词
    fn extract_keywords(item: &ContentItem) -> Vec<String> {
        let mut keywords = Vec::new();

        // 从标题提取
        for word in item.title.split_whitespace() {
            if word.len() > 1 {
                keywords.push(word.to_string());
            }
        }

        // 从描述提取
        if let Some(desc) = &item.description {
            for word in desc.split_whitespace() {
                if word.len() > 1 {
                    keywords.push(word.to_string());
                }
            }
        }

        // 添加分类
        keywords.push(item.category.clone());

        keywords
    }

    /// 获取相关文章推荐
    pub fn get_recommendations(&self, current_url: &str, limit: usize) -> Vec<RecommendedItem> {
        let mut scores: HashMap<String, f64> = HashMap::new();
        let current_item = match self.find_item_by_url(current_url) {
            Some(item) => item,
            None => return self.get_random_items(limit),
        };

        // 计算当前文章的关键词
        let current_keywords = Self::extract_keywords(current_item);

        // 计算每篇文章的相关性分数
        for item in self.site_content.all_items() {
            // 跳过当前文章
            if item.url == current_url {
                continue;
            }

            // 同分类的文章加基础分
            let mut score = 0.0;
            if item.category == current_item.category {
                score += 2.0;
            }

            // 关键词匹配
            let item_keywords = Self::extract_keywords(item);
            for kw in &current_keywords {
                if item_keywords
                    .iter()
                    .any(|ik| ik.to_lowercase() == kw.to_lowercase())
                {
                    score += 1.0;
                }
            }

            // 描述相似度
            if let (Some(cur_desc), Some(item_desc)) =
                (&current_item.description, &item.description)
            {
                if Self::text_similarity(cur_desc, item_desc) > 0.3 {
                    score += Self::text_similarity(cur_desc, item_desc) * 2.0;
                }
            }

            if score > 0.0 {
                scores.insert(item.url.clone(), score);
            }
        }

        // 按分数排序并收集
        let mut recommendations: Vec<_> = scores
            .iter()
            .filter_map(|(url, score)| {
                self.find_item_by_url(url).map(|item| RecommendedItem {
                    url: item.url.clone(),
                    title: item.title.clone(),
                    category: item.category.clone(),
                    category_name: self.get_category_name(&item.category),
                    description: item.description.clone(),
                    score: *score,
                })
            })
            .collect();

        // 按分类去重，保留每分类分数最高的
        let mut seen_categories: HashMap<String, RecommendedItem> = HashMap::new();
        let mut deduped = Vec::new();

        for item in recommendations {
            if let Some(existing) = seen_categories.get(&item.category) {
                if item.score > existing.score {
                    seen_categories.insert(item.category.clone(), item.clone());
                    deduped.retain(|i: &RecommendedItem| i.category != item.category);
                    deduped.push(item);
                }
            } else {
                seen_categories.insert(item.category.clone(), item.clone());
                deduped.push(item);
            }
        }

        // 重新排序并限制数量
        deduped.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        deduped.into_iter().take(limit).collect()
    }

    /// 获取热门文章
    pub fn get_popular_items(&self, limit: usize) -> Vec<RecommendedItem> {
        let items: Vec<_> = self
            .site_content
            .all_items()
            .into_iter()
            .map(|item| RecommendedItem {
                url: item.url.clone(),
                title: item.title.clone(),
                category: item.category.clone(),
                category_name: self.get_category_name(&item.category),
                description: item.description.clone(),
                score: 0.0,
            })
            .take(limit * 2)
            .collect();

        // 按分类去重
        let mut seen_categories = std::collections::HashSet::new();
        items
            .into_iter()
            .filter(|item| seen_categories.insert(item.category.clone()))
            .take(limit)
            .collect()
    }

    /// 获取最新文章
    pub fn get_latest_items(&self, limit: usize) -> Vec<RecommendedItem> {
        let mut items: Vec<_> = self
            .site_content
            .all_items()
            .into_iter()
            .filter(|item| item.date.is_some())
            .take(limit * 2)
            .map(|item| RecommendedItem {
                url: item.url.clone(),
                title: item.title.clone(),
                category: item.category.clone(),
                category_name: self.get_category_name(&item.category),
                description: item.description.clone(),
                score: 0.0,
            })
            .collect();

        // 按标题排序（暂时用标题排序代替日期排序）
        items.sort_by(|a, b| b.title.cmp(&a.title));

        // 按分类去重
        let mut seen_categories = std::collections::HashSet::new();
        items
            .into_iter()
            .filter(|item| seen_categories.insert(item.category.clone()))
            .take(limit)
            .collect()
    }

    /// 根据 URL 查找文章
    fn find_item_by_url(&self, url: &str) -> Option<&ContentItem> {
        self.site_content
            .all_items()
            .into_iter()
            .find(|item| &item.url == url)
    }

    /// 获取分类名称
    fn get_category_name(&self, category_url: &str) -> String {
        self.site_content
            .categories
            .iter()
            .find(|cat| cat.url == category_url)
            .map(|cat| cat.name.clone())
            .unwrap_or_else(|| category_url.to_string())
    }

    /// 计算文本相似度（简单的词重叠）
    fn text_similarity(text1: &str, text2: &str) -> f64 {
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count() as f64;
        let union = words1.union(&words2).count() as f64;

        intersection / union
    }

    /// 获取随机文章
    fn get_random_items(&self, limit: usize) -> Vec<RecommendedItem> {
        let items: Vec<_> = self
            .site_content
            .all_items()
            .into_iter()
            .map(|item| RecommendedItem {
                url: item.url.clone(),
                title: item.title.clone(),
                category: item.category.clone(),
                category_name: self.get_category_name(&item.category),
                description: item.description.clone(),
                score: 0.0,
            })
            .take(limit)
            .collect();

        items
    }
}

/// 为 SiteContent 添加辅助方法
impl SiteContent {
    /// 获取所有文章的迭代器
    pub fn all_items(&self) -> Vec<&ContentItem> {
        self.categories
            .iter()
            .flat_map(|cat| cat.items.iter())
            .collect()
    }
}
