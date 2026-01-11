use crate::config::Config;
use crate::scanner::{Category, ContentItem, SiteContent};
use tera::{Context, Tera};

pub struct TemplateRenderer {
    tera: Tera,
    config: Config,
}

impl TemplateRenderer {
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Create templates directory if it doesn't exist
        if !config.paths.templates_dir.exists() {
            std::fs::create_dir_all(&config.paths.templates_dir)?;
        }

        // Create Tera instance
        let pattern = format!("{}/**/*.html", config.paths.templates_dir.display());
        log::debug!("Looking for templates with pattern: {}", pattern);

        let tera = match Tera::new(&pattern) {
            Ok(mut t) => {
                log::info!("Loaded templates from directory");
                // Check if any templates were actually loaded
                if t.get_template_names().count() == 0 {
                    log::warn!("No templates found in directory, using built-in templates");
                    t = Self::create_builtin_templates()?;
                }
                t
            },
            Err(e) => {
                log::warn!("Failed to load templates: {}", e);
                log::info!("Using built-in templates");
                Self::create_builtin_templates()?
            }
        };

        Ok(Self {
            tera,
            config: config.clone(),
        })
    }

    fn create_builtin_templates() -> Result<Tera, Box<dyn std::error::Error>> {
        let mut tera = Tera::default();

        // Base template with lazy loading, KaTeX, and i18n support
        let base_template = [
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>"#,
            r#"{% block title %}"#,
            r#"{{ config.site.title }}"#,
            r#"{% endblock title %}</title>
    <meta name="description" content="{% block description %}{{ config.site.description }}{% endblock description %}">
    <!-- KaTeX CSS for math formula rendering -->
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
    <style>
        /* CSS Variables for theming */
        :root {
            --bg-color: #ffffff;
            --fg-color: #333333;
            --primary-color: #3498db;
            --secondary-color: #2c3e50;
            --accent-color: #e74c3c;
            --border-color: #e0e0e0;
            --muted-color: #666666;
            --code-bg: #f5f5f5;
            --card-bg: #ffffff;
            --header-bg: #2c3e50;
            --footer-bg: #f8f9fa;
        }
        
        [data-theme="dark"] {
            --bg-color: #1a1a1a;
            --fg-color: #e0e0e0;
            --primary-color: #5dade2;
            --secondary-color: #34495e;
            --accent-color: #ec7063;
            --border-color: #333333;
            --muted-color: #999999;
            --code-bg: #2d2d2d;
            --card-bg: #242424;
            --header-bg: #1a1a2e;
            --footer-bg: #1a1a1a;
        }

        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif; line-height: 1.6; color: var(--fg-color); background: var(--bg-color); transition: background-color 0.3s, color 0.3s; }
        .container { max-width: 1200px; margin: 0 auto; padding: 0 20px; }
        header { background: var(--header-bg); color: #fff; padding: 20px 0; transition: background-color 0.3s; }
        header h1 { font-size: 2em; }
        nav { display: flex; align-items: center; gap: 20px; flex-wrap: wrap; }
        nav a { color: #fff; text-decoration: none; transition: opacity 0.2s; }
        nav a:hover { opacity: 0.8; text-decoration: underline; }
        main { padding: 40px 0; min-height: calc(100vh - 200px); }
        footer { background: var(--footer-bg); text-align: center; padding: 20px 0; color: var(--muted-color); transition: background-color 0.3s; }
        .post-list { display: grid; gap: 20px; }
        .post-item { background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 8px; padding: 20px; transition: background-color 0.3s, border-color 0.3s; }
        .post-item h2 { margin-bottom: 10px; }
        .post-item a { color: var(--primary-color); text-decoration: none; }
        .post-item a:hover { text-decoration: underline; }
        .content { max-width: 800px; margin: 0 auto; }
        .content h1 { font-size: 2.5em; margin-bottom: 20px; }
        .content h2 { font-size: 2em; margin-top: 40px; margin-bottom: 15px; }
        .content h3 { font-size: 1.5em; margin-top: 30px; margin-bottom: 12px; }
        .content p { margin-bottom: 15px; }
        .content img { max-width: 100%; height: auto; border-radius: 8px; margin: 20px 0; }
        .content pre { background: var(--code-bg); padding: 15px; border-radius: 8px; overflow-x: auto; margin: 20px 0; }
        .content code { background: var(--code-bg); padding: 2px 6px; border-radius: 4px; font-family: 'Fira Code', 'Consolas', monospace; font-size: 0.9em; }
        .content pre code { background: none; padding: 0; }
        .meta { color: var(--muted-color); font-size: 0.9em; margin-bottom: 20px; }
        .category-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 20px; }
        .category-card { background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 8px; padding: 20px; transition: background-color 0.3s, border-color 0.3s; }
        .category-card h3 { margin-bottom: 10px; }
        .category-card a { color: var(--primary-color); text-decoration: none; }
        .category-card a:hover { text-decoration: underline; }
        
        /* Lazy loading image styles */
        .content img[lazy="true"] {
            opacity: 0;
            transition: opacity 0.3s ease-in;
        }
        .content img[lazy="true"].loaded {
            opacity: 1;
        }
        .content img.placeholder {
            background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
            background-size: 200% 100%;
            animation: shimmer 1.5s infinite;
        }
        @keyframes shimmer {
            0% { background-position: 200% 0; }
            100% { background-position: -200% 0; }
        }
        
        /* KaTeX math formula styles */
        .katex-display { overflow-x: auto; overflow-y: hidden; padding: 10px 0; margin: 15px 0; }
        .katex-display > .katex { white-space: nowrap; }
        .content .katex { font-size: 1.1em; }
        
        /* Theme Toggle Button */
        .theme-toggle {
            background: none;
            border: 2px solid rgba(255, 255, 255, 0.3);
            color: #fff;
            padding: 8px 16px;
            border-radius: 20px;
            cursor: pointer;
            font-size: 0.9em;
            display: flex;
            align-items: center;
            gap: 8px;
            transition: all 0.3s;
        }
        .theme-toggle:hover {
            background: rgba(255, 255, 255, 0.1);
            border-color: rgba(255, 255, 255, 0.5);
        }
        .theme-toggle svg { width: 18px; height: 18px; }
        
        /* Language Switcher */
        .lang-switcher {
            background: none;
            border: 2px solid rgba(255, 255, 255, 0.3);
            color: #fff;
            padding: 6px 12px;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.9em;
            transition: all 0.3s;
        }
        .lang-switcher:hover {
            background: rgba(255, 255, 255, 0.1);
            border-color: rgba(255, 255, 255, 0.5);
        }
        
        /* Responsive adjustments */
        @media (max-width: 768px) {
            header .container { flex-direction: column; gap: 15px; }
            nav { justify-content: center; }
            .category-list { grid-template-columns: 1fr; }
            .content h1 { font-size: 2em; }
            .content h2 { font-size: 1.5em; }
        }
    </style>
</head>
<body>
    <header>
        <div class="container">
            <h1>"#,
            r#"{{ config.site.title }}"#,
            r#"</h1>
            <nav>
                <a href="/">首页</a>
                <a href="/archive">归档</a>
                <a href="/search" class="search-link" title="搜索" style="display: flex; align-items: center; gap: 5px;">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="18" height="18">
                        <circle cx="11" cy="11" r="8"></circle>
                        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                    </svg>
                    搜索
                </a>
                <select class="lang-switcher" id="langSwitcher" onchange="changeLanguage(this.value)" title="切换语言">
                    <option value="zh-CN">中文</option>
                    <option value="en">English</option>
                </select>
                <button class="theme-toggle" id="themeToggle" title="切换主题">
                    <svg class="sun-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="display:none;">
                        <circle cx="12" cy="12" r="5"></circle>
                        <line x1="12" y1="1" x2="12" y2="3"></line>
                        <line x1="12" y1="21" x2="12" y2="23"></line>
                        <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
                        <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
                        <line x1="1" y1="12" x2="3" y2="12"></line>
                        <line x1="21" y1="12" x2="23" y2="12"></line>
                        <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
                        <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
                    </svg>
                    <svg class="moon-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
                    </svg>
                    <span class="theme-text">深色</span>
                </button>
            </nav>
        </div>
    </header>
    <main class="container">
        {% block content %}{% endblock content %}
    </main>
    <footer>
        <div class="container">
            <p>&copy; 2026 {{ config.site.author }}. 版权所有.</p>
        </div>
    </footer>
    
    <!-- KaTeX JS for math formula rendering -->
    <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js"></script>
    
    <script>
        // ==================== Image Lazy Loading ====================
        function initLazyLoading() {
            var images = document.querySelectorAll('img[lazy="true"]');
            
            if ('IntersectionObserver' in window) {
                var imageObserver = new IntersectionObserver(function(entries, observer) {
                    entries.forEach(function(entry) {
                        if (entry.isIntersecting) {
                            var img = entry.target;
                            var preloadImg = new Image();
                            preloadImg.onload = function() {
                                img.src = img.dataset.src;
                                img.classList.add('loaded');
                                img.removeAttribute('lazy');
                                img.removeAttribute('data-src');
                            };
                            preloadImg.onerror = function() {
                                img.src = img.dataset.src;
                                img.classList.add('loaded');
                            };
                            preloadImg.src = img.dataset.src;
                            observer.unobserve(img);
                        }
                    });
                }, {
                    rootMargin: '50px 0px',
                    threshold: 0.01
                });
                
                images.forEach(function(img) {
                    imageObserver.observe(img);
                });
            } else {
                images.forEach(function(img) {
                    img.src = img.dataset.src;
                    img.classList.add('loaded');
                });
            }
        }
        
        // Add lazy loading to all content images on page load
        document.addEventListener('DOMContentLoaded', function() {
            var contentImages = document.querySelectorAll('.content img');
            contentImages.forEach(function(img) {
                if (!img.hasAttribute('lazy')) {
                    img.setAttribute('lazy', 'true');
                    img.classList.add('placeholder');
                    var originalSrc = img.src;
                    img.removeAttribute('src');
                    img.setAttribute('data-src', originalSrc);
                }
            });
            initLazyLoading();
        });
        
        // ==================== KaTeX Math Rendering ====================
        function renderMath() {
            var displayMathElements = document.querySelectorAll('.katex-display');
            displayMathElements.forEach(function(el) {
                try {
                    if (!el.classList.contains('katex-rendered')) {
                        katex.render(el.textContent, el, {
                            throwOnError: false,
                            displayMode: true,
                            output: 'html'
                        });
                        el.classList.add('katex-rendered');
                    }
                } catch (e) {
                    console.error('KaTeX rendering error:', e);
                }
            });
        }
        
        if (typeof katex !== 'undefined') {
            document.addEventListener('DOMContentLoaded', renderMath);
            var themeToggle = document.getElementById('themeToggle');
            if (themeToggle) {
                themeToggle.addEventListener('click', function() {
                    setTimeout(renderMath, 100);
                });
            }
        }
        
        // ==================== Theme Toggle ====================
        function initTheme() {
            var themeToggle = document.getElementById('themeToggle');
            if (!themeToggle) return;
            
            var savedTheme = localStorage.getItem('theme');
            var prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            
            if (savedTheme === 'dark' || (!savedTheme && prefersDark)) {
                document.documentElement.setAttribute('data-theme', 'dark');
                updateThemeButton(true);
            } else {
                document.documentElement.setAttribute('data-theme', 'light');
                updateThemeButton(false);
            }
        }
        
        function toggleTheme() {
            var currentTheme = document.documentElement.getAttribute('data-theme');
            var newTheme = currentTheme === 'dark' ? 'light' : 'dark';
            
            document.documentElement.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);
            updateThemeButton(newTheme === 'dark');
        }
        
        function updateThemeButton(isDark) {
            var themeToggle = document.getElementById('themeToggle');
            if (!themeToggle) return;
            
            var sunIcon = themeToggle.querySelector('.sun-icon');
            var moonIcon = themeToggle.querySelector('.moon-icon');
            var themeText = themeToggle.querySelector('.theme-text');
            
            if (isDark) {
                sunIcon.style.display = 'block';
                moonIcon.style.display = 'none';
                themeText.textContent = '浅色';
            } else {
                sunIcon.style.display = 'none';
                moonIcon.style.display = 'block';
                themeText.textContent = '深色';
            }
        }
        
        // ==================== Language Switcher ====================
        function initLanguageSwitcher() {
            var langSwitcher = document.getElementById('langSwitcher');
            if (!langSwitcher) return;
            
            var savedLang = localStorage.getItem('language');
            if (savedLang) {
                langSwitcher.value = savedLang;
            }
        }
        
        function changeLanguage(lang) {
            localStorage.setItem('language', lang);
            window.location.reload();
        }
        
        // ==================== Initialize Everything ====================
        document.addEventListener('DOMContentLoaded', function() {
            var themeToggle = document.getElementById('themeToggle');
            if (themeToggle) {
                themeToggle.addEventListener('click', toggleTheme);
                initTheme();
            }
            
            initLanguageSwitcher();
            
            if (typeof katex !== 'undefined') {
                setTimeout(renderMath, 100);
            }
        });
    </script>
</body>
</html>"#,
        ].join("");

        tera.add_raw_template("base.html", &base_template)?;

        // Index template
        tera.add_raw_template(
            "index.html",
            r#"{% extends "base.html" %}
{% block title %}{{ config.site.title }} - 首页{% endblock title %}
{% block content %}
<div class="content">
    <h2>{{ config.site.description }}</h2>
    
    <!-- Popular Content Section -->
    <div id="popular-content" style="margin-top: 60px;">
        <h2 style="margin-bottom: 30px;">热门文章</h2>
        <div id="popular-list">
            <p style="color: #888; text-align: center; padding: 40px;">加载热门内容中...</p>
        </div>
    </div>
    
    <!-- Latest Content Section -->
    <div id="latest-content" style="margin-top: 60px;">
        <h2 style="margin-bottom: 30px;">最新文章</h2>
        <div id="latest-list">
            <p style="color: #888; text-align: center; padding: 40px;">加载最新内容中...</p>
        </div>
    </div>
    
    <h2 style="margin-top: 60px;">分类</h2>
    <div class="category-list">
        {% for category in site_content.categories %}
        <div class="category-card">
            <h3><a href="/{{ category.url }}">{{ category.name }}</a></h3>
            <p>{{ category.description | default(value="") }}</p>
            <p>共 {{ category.items | length }} 篇</p>
        </div>
        {% endfor %}
    </div>
</div>

<script>
document.addEventListener('DOMContentLoaded', function() {
    // Load popular content
    loadPopularContent();
    
    // Load latest content
    loadLatestContent();
    
    async function loadPopularContent() {
        try {
            const response = await fetch('/api/popular-content');
            const data = await response.json();
            
            if (data.status === 'success') {
                const popularList = document.getElementById('popular-list');
                
                if (data.popular_items && data.popular_items.length > 0) {
                    const html = `
                        <div class="post-list">
                            ${data.popular_items.map(item => `
                                <div class="post-item">
                                    <h3><a href="/${item.url}">${item.title}</a></h3>
                                    <div class="meta">
                                        <span>分类: <a href="/${item.category}">${item.category_name}</a></span>
                                    </div>
                                    ${item.description ? `<p>${item.description.length > 120 ? item.description.substring(0, 120) + '...' : item.description}</p>` : ''}
                                </div>
                            `).join('')}
                        </div>
                    `;
                    
                    popularList.innerHTML = html;
                } else {
                    popularList.innerHTML = '<p style="color: #888; text-align: center; padding: 40px;">暂无热门内容</p>';
                }
            }
        } catch (error) {
            console.error('加载热门内容失败:', error);
            const popularList = document.getElementById('popular-list');
            popularList.innerHTML = '<p style="color: #c00; text-align: center; padding: 40px;">加载失败</p>';
        }
    }
    
    async function loadLatestContent() {
        try {
            const response = await fetch('/api/latest-content');
            const data = await response.json();
            
            if (data.status === 'success') {
                const latestList = document.getElementById('latest-list');
                
                if (data.latest_items && data.latest_items.length > 0) {
                    const html = `
                        <div class="post-list">
                            ${data.latest_items.map(item => `
                                <div class="post-item">
                                    <h3><a href="/${item.url}">${item.title}</a></h3>
                                    <div class="meta">
                                        <span>分类: <a href="/${item.category}">${item.category_name}</a></span>
                                    </div>
                                    ${item.description ? `<p>${item.description.length > 120 ? item.description.substring(0, 120) + '...' : item.description}</p>` : ''}
                                </div>
                            `).join('')}
                        </div>
                    `;
                    
                    latestList.innerHTML = html;
                } else {
                    latestList.innerHTML = '<p style="color: #888; text-align: center; padding: 40px;">暂无最新内容</p>';
                }
            }
        } catch (error) {
            console.error('加载最新内容失败:', error);
            const latestList = document.getElementById('latest-list');
            latestList.innerHTML = '<p style="color: #c00; text-align: center; padding: 40px;">加载失败</p>';
        }
    }
});
</script>
{% endblock content %}"#,
        )?;

        // Category template
        tera.add_raw_template(
            "category.html",
            r#"{% extends "base.html" %}
{% block title %}{{ category.name }} - {{ config.site.title }}{% endblock title %}
{% block description %}{{ category.description | default(value=category.name) }}{% endblock description %}
{% block content %}
<div class="content">
    <h1>{{ category.name }}</h1>
    {% if category.description %}
    <p>{{ category.description }}</p>
    {% endif %}
    <div class="post-list">
        {% for item in category.items %}
        <div class="post-item">
            <h2><a href="/{{ item.url }}">{{ item.title }}</a></h2>
            <div class="meta">
                {% if item.date %}
                <span>{{ item.date }}</span>
                {% endif %}
            </div>
            <p>{{ item.description | default(value="") }}</p>
        </div>
        {% endfor %}
    </div>
</div>
{% endblock content %}"#,
        )?;

        // Item template
        tera.add_raw_template(
            "item.html",
            r#"{% extends "base.html" %}
{% block title %}{{ item.title }} - {{ config.site.title }}{% endblock title %}
{% block description %}{{ item.description | default(value=item.title) }}{% endblock description %}
{% block content %}
<div class="content">
    <h1>{{ item.title }}</h1>
    <div class="meta">
        {% if item.date %}
        <span>发布于 {{ item.date }}</span>
        {% endif %}
        {% if item.author %}
        <span>作者: {{ item.author }}</span>
        {% endif %}
    </div>
    <article>
        {{ item.html_content | safe }}
    </article>
    
    <!-- Comments Section -->
    <div id="comments" style="margin-top: 60px; border-top: 1px solid var(--border-color); padding-top: 40px;">
        <h2 style="margin-bottom: 30px;">评论 <span id="comment-count"></span></h2>
        
        <!-- Add Comment Form -->
        <div class="comment-form" style="margin-bottom: 40px;">
            <h3 style="margin-bottom: 20px;">发表评论</h3>
            <form id="commentForm">
                <div class="form-group" style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 15px;">
                    <div>
                        <label for="author" style="display: block; margin-bottom: 8px; font-weight: 500;">昵称 *</label>
                        <input type="text" id="author" name="author" required style="width: 100%; padding: 10px; border: 1px solid var(--border-color); border-radius: 4px;">
                    </div>
                    <div>
                        <label for="website" style="display: block; margin-bottom: 8px; font-weight: 500;">网站 (可选)</label>
                        <input type="url" id="website" name="website" style="width: 100%; padding: 10px; border: 1px solid var(--border-color); border-radius: 4px;">
                    </div>
                </div>
                <div class="form-group" style="margin-bottom: 20px;">
                    <label for="content" style="display: block; margin-bottom: 8px; font-weight: 500;">评论内容 *</label>
                    <textarea id="content" name="content" required rows="4" style="width: 100%; padding: 10px; border: 1px solid var(--border-color); border-radius: 4px;"></textarea>
                </div>
                <button type="submit" class="btn" style="background: var(--primary-color);">发表评论</button>
            </form>
            <div id="comment-message" style="margin-top: 15px; display: none;"></div>
        </div>
        
        <!-- Comments List -->
        <div id="comments-list">
            <p style="color: #888; text-align: center; padding: 40px;">加载评论中...</p>
        </div>
    </div>
    
    <!-- Recommendations Section -->
    <div id="recommendations" style="margin-top: 60px; border-top: 1px solid var(--border-color); padding-top: 40px;">
        <h2 style="margin-bottom: 30px;">相关推荐</h2>
        <div id="recommendations-list">
            <p style="color: #888; text-align: center; padding: 40px;">加载推荐内容中...</p>
        </div>
    </div>
    
    <p style="margin-top: 40px;"><a href="/{{ item.category }}">← 返回 {{ item.category }}</a></p>
</div>

<script>
// Comments functionality
document.addEventListener('DOMContentLoaded', function() {
    const currentSlug = window.location.pathname.substring(1).replace('.html', '');
    
    // Load comments
    loadComments(currentSlug);
    
    // Load recommendations
    loadRecommendations(currentSlug);
    
    // Handle comment form submission
    const commentForm = document.getElementById('commentForm');
    if (commentForm) {
        commentForm.addEventListener('submit', async function(e) {
            e.preventDefault();
            
            const submitBtn = commentForm.querySelector('button[type="submit"]');
            const originalText = submitBtn.textContent;
            submitBtn.textContent = '提交中...';
            submitBtn.disabled = true;
            
            const formData = {
                slug: currentSlug,
                author: document.getElementById('author').value.trim(),
                content: document.getElementById('content').value.trim(),
                website: document.getElementById('website').value.trim() || null
            };
            
            const messageDiv = document.getElementById('comment-message');
            messageDiv.style.display = 'none';
            
            try {
                const response = await fetch('/api/comments', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify(formData)
                });
                
                const data = await response.json();
                
                if (response.ok && data.status === 'success') {
                    messageDiv.style.display = 'block';
                    messageDiv.style.color = '#155724';
                    messageDiv.style.background = '#d4edda';
                    messageDiv.style.padding = '15px';
                    messageDiv.style.borderRadius = '4px';
                    messageDiv.textContent = '评论发表成功！刷新页面查看您的评论。';
                    commentForm.reset();
                    
                    // Reload comments after a short delay
                    setTimeout(() => {
                        loadComments(currentSlug);
                    }, 2000);
                } else {
                    messageDiv.style.display = 'block';
                    messageDiv.style.color = '#721c24';
                    messageDiv.style.background = '#f8d7da';
                    messageDiv.style.padding = '15px';
                    messageDiv.style.borderRadius = '4px';
                    messageDiv.textContent = '评论发表失败: ' + (data.message || '未知错误');
                }
            } catch (error) {
                messageDiv.style.display = 'block';
                messageDiv.style.color = '#721c24';
                messageDiv.style.background = '#f8d7da';
                messageDiv.style.padding = '15px';
                messageDiv.style.borderRadius = '4px';
                messageDiv.textContent = '请求失败: ' + error.message;
            }
            
            submitBtn.textContent = originalText;
            submitBtn.disabled = false;
        });
    }
    
    async function loadComments(slug) {
        try {
            const response = await fetch('/api/comments/' + slug);
            const data = await response.json();
            
            if (data.status === 'success') {
                const commentsList = document.getElementById('comments-list');
                const commentCount = document.getElementById('comment-count');
                
                if (data.comments && data.comments.length > 0) {
                    commentCount.textContent = '(' + data.count + ')';
                    
                    const html = data.comments.map(comment => `
                        <div class="comment" style="margin-bottom: 30px; padding-bottom: 30px; border-bottom: 1px solid var(--border-color);">
                            <div class="comment-header" style="display: flex; justify-content: space-between; margin-bottom: 15px;">
                                <strong style="color: var(--primary-color);">${comment.author}</strong>
                                <span style="color: #888; font-size: 0.9em;">${new Date(comment.created_at).toLocaleDateString('zh-CN')}</span>
                            </div>
                            <div class="comment-content" style="line-height: 1.6;">${comment.content.replace(/\n/g, '<br>')}</div>
                            ${comment.website ? `<div style="margin-top: 10px;"><a href="${comment.website}" target="_blank" style="color: #888; font-size: 0.9em;">${comment.website}</a></div>` : ''}
                        </div>
                    `).join('');
                    
                    commentsList.innerHTML = html;
                } else {
                    commentCount.textContent = '(0)';
                    commentsList.innerHTML = '<p style="color: #888; text-align: center; padding: 40px;">暂无评论，快来发表第一条评论吧！</p>';
                }
            }
        } catch (error) {
            console.error('加载评论失败:', error);
            const commentsList = document.getElementById('comments-list');
            commentsList.innerHTML = '<p style="color: #c00; text-align: center; padding: 40px;">加载评论失败</p>';
        }
    }
    
    async function loadRecommendations(slug) {
        try {
            const response = await fetch('/api/recommendations/' + slug);
            const data = await response.json();
            
            if (data.status === 'success') {
                const recommendationsList = document.getElementById('recommendations-list');
                
                if (data.recommendations && data.recommendations.length > 0) {
                    const html = `
                        <div class="recommendations-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px;">
                            ${data.recommendations.map(item => `
                                <div class="recommendation-item" style="background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 8px; padding: 20px; transition: all 0.3s;">
                                    <h3 style="margin-bottom: 10px;"><a href="/${item.url}" style="color: var(--primary-color); text-decoration: none;">${item.title}</a></h3>
                                    <div class="meta" style="color: #888; font-size: 0.9em; margin-bottom: 10px;">
                                        分类: <a href="/${item.category}" style="color: #888;">${item.category_name}</a>
                                    </div>
                                    ${item.description ? `<p style="color: var(--muted-color); font-size: 0.95em; line-height: 1.5;">${item.description.length > 100 ? item.description.substring(0, 100) + '...' : item.description}</p>` : ''}
                                </div>
                            `).join('')}
                        </div>
                    `;
                    
                    recommendationsList.innerHTML = html;
                } else {
                    recommendationsList.innerHTML = '<p style="color: #888; text-align: center; padding: 40px;">暂无推荐内容</p>';
                }
            }
        } catch (error) {
            console.error('加载推荐失败:', error);
            const recommendationsList = document.getElementById('recommendations-list');
            recommendationsList.innerHTML = '<p style="color: #c00; text-align: center; padding: 40px;">加载推荐失败</p>';
        }
    }
});
</script>
{% endblock content %}"#,
        )?;

        // Archive template
        tera.add_raw_template(
            "archive.html",
            r#"{% extends "base.html" %}
{% block title %}文章归档 - {{ config.site.title }}{% endblock title %}
{% block content %}
<div class="content">
    <h1>文章归档</h1>
    <p>共 {{ total_items }} 篇文章</p>
    <div class="post-list" style="margin-top: 30px;">
        {% for item in all_items %}
        <div class="post-item">
            <h2><a href="/{{ item.url }}">{{ item.title }}</a></h2>
            <div class="meta">
                {% if item.date %}
                <span>{{ item.date }}</span>
                {% endif %}
                <span>分类: {{ item.category_name }}</span>
            </div>
            <p>{{ item.description | default(value="") }}</p>
        </div>
        {% endfor %}
    </div>
</div>
{% endblock content %}"#,
        )?;

        // Search results template
        tera.add_raw_template(
            "search.html",
            r#"{% extends "base.html" %}
{% block title %}搜索: {{ search_query }} - {{ config.site.title }}{% endblock title %}
{% block description %}搜索 {{ search_query }} 的结果{% endblock description %}
{% block content %}
<div class="content">
    <h1>搜索结果</h1>
    
    <!-- Search Form -->
    <div class="search-form" style="margin: 30px 0; padding: 20px; background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 8px;">
        <form action="/search" method="GET" style="display: flex; gap: 10px; flex-wrap: wrap;">
            <input type="text" 
                   name="q" 
                   value="{{ search_query }}" 
                   placeholder="输入关键词搜索..." 
                   style="flex: 1; min-width: 200px; padding: 12px 15px; border: 2px solid var(--border-color); border-radius: 6px; font-size: 1em;"
                   required
                   autofocus>
            <button type="submit" class="btn" style="padding: 12px 25px; background: var(--primary-color); color: #fff; border: none; border-radius: 6px; cursor: pointer; font-size: 1em;">
                搜索
            </button>
        </form>
    </div>
    
    <!-- Search Results Summary -->
    <div class="search-summary" style="margin-bottom: 30px; padding-bottom: 20px; border-bottom: 1px solid var(--border-color);">
        {% if search_query %}
            {% if search_results | length > 0 %}
                <p style="color: var(--muted-color);">
                    关于 "<strong>{{ search_query }}</strong>" 的搜索结果，共找到 <strong>{{ total_count }}</strong> 条结果
                </p>
            {% else %}
                <p style="color: var(--muted-color);">
                    没有找到与 "<strong>{{ search_query }}</strong>" 相关的内容
                </p>
            {% endif %}
        {% else %}
            <p style="color: var(--muted-color);">请输入关键词进行搜索</p>
        {% endif %}
    </div>
    
    <!-- Search Results -->
    {% if search_results | length > 0 %}
    <div class="search-results">
        {% for result in search_results %}
        <div class="search-result-item" style="margin-bottom: 25px; padding-bottom: 25px; border-bottom: 1px solid var(--border-color);">
            <h2 style="margin-bottom: 10px;">
                <a href="/{{ result.slug }}" style="color: var(--primary-color); text-decoration: none;">
                    {{ result.title }}
                </a>
            </h2>
            <div class="meta" style="margin-bottom: 10px; font-size: 0.9em;">
                {% if result.category %}
                <span style="color: var(--muted-color);">
                    分类: <a href="/{{ result.category }}" style="color: var(--primary-color);">{{ result.category }}</a>
                </span>
                {% endif %}
            </div>
            {% if result.snippet %}
            <p class="snippet" style="color: var(--fg-color); line-height: 1.6; margin-bottom: 10px;">
                {{ result.snippet | safe }}
            </p>
            {% endif %}
            <a href="/{{ result.slug }}" style="color: var(--primary-color); font-size: 0.9em;">查看详情 &rarr;</a>
        </div>
        {% endfor %}
    </div>
    {% endif %}
    
    <!-- Quick Links -->
    {% if search_query == "" %}
    <div class="search-tips" style="margin-top: 40px; padding: 30px; background: var(--card-bg); border: 1px solid var(--border-color); border-radius: 8px;">
        <h2 style="margin-bottom: 20px;">搜索提示</h2>
        <ul style="list-style: none; padding: 0;">
            <li style="margin-bottom: 15px; padding-left: 20px; position: relative;">
                <span style="position: absolute; left: 0; color: var(--primary-color);">•</span>
                输入准确的关键词可以获得更精确的结果
            </li>
            <li style="margin-bottom: 15px; padding-left: 20px; position: relative;">
                <span style="position: absolute; left: 0; color: var(--primary-color);">•</span>
                支持按标题和内容进行搜索
            </li>
            <li style="margin-bottom: 15px; padding-left: 20px; position: relative;">
                <span style="position: absolute; left: 0; color: var(--primary-color);">•</span>
                尝试使用更短或更简单的关键词
            </li>
        </ul>
    </div>
    {% endif %}
</div>
{% endblock content %}"#,
        )?;

        // Admin login template
        tera.add_raw_template(
            "admin_login.html",
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>管理后台 - {{ config.site.title }}</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .login-container {
            background: #fff;
            padding: 40px;
            border-radius: 10px;
            box-shadow: 0 15px 35px rgba(0, 0, 0, 0.2);
            width: 100%;
            max-width: 400px;
        }
        .login-container h1 {
            text-align: center;
            margin-bottom: 30px;
            color: #333;
            font-size: 1.8em;
        }
        .form-group {
            margin-bottom: 20px;
        }
        .form-group label {
            display: block;
            margin-bottom: 8px;
            color: #555;
            font-weight: 500;
        }
        .form-group input {
            width: 100%;
            padding: 12px 15px;
            border: 2px solid #e0e0e0;
            border-radius: 6px;
            font-size: 1em;
            transition: border-color 0.3s;
        }
        .form-group input:focus {
            outline: none;
            border-color: #667eea;
        }
        .btn {
            width: 100%;
            padding: 12px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
            border: none;
            border-radius: 6px;
            font-size: 1em;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
        }
        .error {
            background: #fee;
            color: #c00;
            padding: 10px;
            border-radius: 6px;
            margin-bottom: 20px;
            text-align: center;
        }
        .site-name {
            text-align: center;
            margin-bottom: 10px;
            color: #888;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="login-container">
        <h1>管理后台</h1>
        <p class="site-name">{{ config.site.title }}</p>
        {% if error %}
        <div class="error">{{ error }}</div>
        {% endif %}
        <form method="POST" action="/admin/login">
            <div class="form-group">
                <label for="password">密码</label>
                <input type="password" id="password" name="password" required autofocus>
            </div>
            <button type="submit" class="btn">登录</button>
        </form>
    </div>
</body>
</html>"#,
        )?;

        // Admin base template
        tera.add_raw_template(
            "admin_base.html",
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}管理后台{% endblock %} - {{ config.site.title }}</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: #f5f5f5;
            min-height: 100vh;
        }
        .admin-header {
            background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%);
            color: #fff;
            padding: 15px 30px;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
        }
        .admin-header h1 {
            font-size: 1.5em;
        }
        .admin-header a {
            color: #fff;
            text-decoration: none;
            padding: 8px 16px;
            border-radius: 4px;
            transition: background 0.3s;
        }
        .admin-header a:hover {
            background: rgba(255, 255, 255, 0.1);
        }
        .admin-container {
            display: flex;
            min-height: calc(100vh - 60px);
        }
        .admin-sidebar {
            width: 220px;
            background: #fff;
            border-right: 1px solid #e0e0e0;
            padding: 20px 0;
        }
        .admin-sidebar a {
            display: block;
            padding: 12px 25px;
            color: #555;
            text-decoration: none;
            transition: all 0.3s;
            border-left: 3px solid transparent;
        }
        .admin-sidebar a:hover,
        .admin-sidebar a.active {
            background: #f8f9fa;
            color: #3498db;
            border-left-color: #3498db;
        }
        .admin-content {
            flex: 1;
            padding: 30px;
            overflow-x: auto;
        }
        .card {
            background: #fff;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.05);
            padding: 25px;
            margin-bottom: 20px;
        }
        .card h2 {
            margin-bottom: 20px;
            color: #333;
            font-size: 1.3em;
            border-bottom: 1px solid #eee;
            padding-bottom: 15px;
        }
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
        }
        .stat-card {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
            padding: 25px;
            border-radius: 8px;
            text-align: center;
        }
        .stat-card h3 {
            font-size: 2.5em;
            margin-bottom: 10px;
        }
        .stat-card p {
            opacity: 0.9;
        }
        .btn {
            display: inline-block;
            padding: 10px 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #fff;
            border: none;
            border-radius: 6px;
            text-decoration: none;
            cursor: pointer;
            font-size: 0.95em;
            transition: transform 0.2s, box-shadow 0.2s;
        }
        .btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
        }
        .btn-sm {
            padding: 6px 12px;
            font-size: 0.85em;
        }
        .btn-danger {
            background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
        }
        .btn-success {
            background: linear-gradient(135deg, #27ae60 0%, #229954 100%);
        }
        .table {
            width: 100%;
            border-collapse: collapse;
        }
        .table th,
        .table td {
            padding: 12px 15px;
            text-align: left;
            border-bottom: 1px solid #eee;
        }
        .table th {
            background: #f8f9fa;
            font-weight: 600;
            color: #555;
        }
        .table tr:hover {
            background: #f8f9fa;
        }
        .form-group {
            margin-bottom: 20px;
        }
        .form-group label {
            display: block;
            margin-bottom: 8px;
            color: #555;
            font-weight: 500;
        }
        .form-group input,
        .form-group textarea,
        .form-group select {
            width: 100%;
            padding: 10px 15px;
            border: 2px solid #e0e0e0;
            border-radius: 6px;
            font-size: 1em;
            transition: border-color 0.3s;
        }
        .form-group input:focus,
        .form-group textarea:focus,
        .form-group select:focus {
            outline: none;
            border-color: #667eea;
        }
        .form-group textarea {
            min-height: 200px;
            resize: vertical;
        }
        .badge {
            display: inline-block;
            padding: 4px 10px;
            border-radius: 20px;
            font-size: 0.85em;
            font-weight: 500;
        }
        .badge-success {
            background: #d4edda;
            color: #155724;
        }
        .badge-warning {
            background: #fff3cd;
            color: #856404;
        }
        .action-btns {
            display: flex;
            gap: 8px;
        }
        .message {
            padding: 15px;
            border-radius: 6px;
            margin-bottom: 20px;
        }
        .message-success {
            background: #d4edda;
            color: #155724;
        }
        .message-error {
            background: #f8d7da;
            color: #721c24;
        }
        .tabs {
            display: flex;
            border-bottom: 1px solid #e0e0e0;
            margin-bottom: 20px;
        }
        .tabs a {
            padding: 12px 20px;
            color: #555;
            text-decoration: none;
            border-bottom: 2px solid transparent;
            transition: all 0.3s;
        }
        .tabs a:hover,
        .tabs a.active {
            color: #3498db;
            border-bottom-color: #3498db;
        }
        .grid-2 {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
        }
        @media (max-width: 768px) {
            .admin-sidebar {
                display: none;
            }
            .grid-2 {
                grid-template-columns: 1fr;
            }
        }
    </style>
</head>
<body>
    <header class="admin-header">
        <h1>{{ config.site.title }} - 管理后台</h1>
        <nav>
            <a href="/" target="_blank">查看站点</a>
            <a href="/admin/logout">退出登录</a>
        </nav>
    </header>
    <div class="admin-container">
        <aside class="admin-sidebar">
            <a href="/admin" class="{% if active == 'overview' %}active{% endif %}">仪表盘</a>
            <a href="/admin/categories" class="{% if active == 'categories' %}active{% endif %}">分类管理</a>
            <a href="/admin/items" class="{% if active == 'items' %}active{% endif %}">文章管理</a>
            <a href="/admin/analytics" class="{% if active == 'analytics' %}active{% endif %}">访问统计</a>
            <a href="/admin/compile" class="{% if active == 'compile' %}active{% endif %}">编译发布</a>
        </aside>
        <main class="admin-content">
            {% block content %}{% endblock %}
        </main>
    </div>
    <script>
        // Auto-hide messages after 3 seconds
        setTimeout(function() {
            var messages = document.querySelectorAll('.message');
            messages.forEach(function(msg) {
                msg.style.transition = 'opacity 0.5s';
                msg.style.opacity = '0';
                setTimeout(function() { msg.remove(); }, 500);
            });
        }, 3000);
    </script>
</body>
</html>"#,
        )?;

        // Admin overview template
        tera.add_raw_template(
            "admin_overview.html",
            r#"{% extends "admin_base.html" %}
{% block title %}仪表盘{% endblock %}
{% block content %}
{% if message %}
<div class="message message-{% if success %}success{% else %}error{% endif %}">{{ message }}</div>
{% endif %}

<div class="card">
    <h2>站点概览</h2>
    <div class="stats-grid">
        <div class="stat-card">
            <h3>{{ overview.total_categories }}</h3>
            <p>分类数量</p>
        </div>
        <div class="stat-card">
            <h3>{{ overview.total_items }}</h3>
            <p>文章数量</p>
        </div>
        <div class="stat-card">
            <h3>{{ overview.total_attachments }}</h3>
            <p>附件数量</p>
        </div>
        <div class="stat-card">
            <h3>{{ overview.total_comments }}</h3>
            <p>评论数量</p>
        </div>
    </div>
</div>

<div class="card">
    <h2>快捷操作</h2>
    <div style="display: flex; gap: 15px; flex-wrap: wrap;">
        <a href="/admin/items/new" class="btn">撰写新文章</a>
        <a href="/admin/categories/new" class="btn">新建分类</a>
        <a href="/admin/compile" class="btn btn-success">编译并发布</a>
    </div>
</div>

{% if last_compiled %}
<div class="card">
    <h2>编译信息</h2>
    <p>上次编译时间: {{ last_compiled }}</p>
</div>
{% endif %}
{% endblock content %}"#,
        )?;

        // Admin categories template
        tera.add_raw_template(
            "admin_categories.html",
            r#"{% extends "admin_base.html" %}
{% block title %}分类管理{% endblock %}
{% block content %}
{% if message %}
<div class="message message-{% if success %}success{% else %}error{% endif %}">{{ message }}</div>
{% endif %}

<div class="card">
    <h2 style="display: flex; justify-content: space-between; align-items: center;">
        分类列表
        <a href="/admin/categories/new" class=" btn">新建分类</a>
    </h2>
    {% if categories | length == 0 %}
    <p style="color: #888; text-align: center; padding: 40px;">暂无分类</p>
    {% else %}
    <table class="table">
        <thead>
            <tr>
                <th>分类名称</th>
                <th>Slug</th>
                <th>文章数量</th>
                <th>操作</th>
            </tr>
        </thead>
        <tbody>
            {% for category in categories %}
            <tr>
                <td>{{ category.title }}</td>
                <td><code>{{ category.slug }}</code></td>
                <td>{{ category.item_count }}</td>
                <td>
                    <div class="action-btns">
                        <a href="/{{ category.slug }}" class="btn btn-sm" target="_blank">查看</a>
                        <form action="/admin/categories/{{ category.slug }}/delete" method="POST" style="display: inline;" onsubmit="return confirm('确定要删除这个分类吗？相关文章也会被删除。')">
                            <button type="submit" class="btn btn-sm btn-danger">删除</button>
                        </form>
                    </div>
                </td>
            </tr>
            {% endfor %}
        </tbody>
    </table>
    {% endif %}
</div>
{% endblock content %}"#,
        )?;

        // Admin new category template
        tera.add_raw_template(
            "admin_new_category.html",
            r#"{% extends "admin_base.html" %}
{% block title %}新建分类{% endblock %}
{% block content %}
<div class="card">
    <h2>新建分类</h2>

    <!-- 上传压缩包表单 -->
    <div style="background: #f8f9fa; padding: 20px; border-radius: 8px; margin-bottom: 20px;">
        <h3 style="margin-bottom: 15px;">方式一：上传压缩包（推荐）</h3>
        <p style="color: #666; margin-bottom: 15px;">
            上传一个 .zip 或 .tar.gz 压缩包，压缩包内应包含分类目录，目录中至少包含 index.md 文件。
        </p>
        <form id="uploadCategoryForm" enctype="multipart/form-data">
            <div class="form-group">
                <label for="category_file">选择压缩包</label>
                <input type="file" id="category_file" name="file" accept=".zip,.tar.gz,.tgz" required>
            </div>
            <button type="submit" class="btn">上传并创建</button>
        </form>
        <div id="uploadResult" style="margin-top: 15px; display: none;"></div>
    </div>

    <div style="text-align: center; margin: 20px 0; color: #999;">— 或者 —</div>

    <!-- 手动创建表单 -->
    <h3 style="margin-bottom: 15px;">方式二：手动创建</h3>
    <form method="POST" action="/admin/categories">
        <div class="form-group">
            <label for="name">分类名称</label>
            <input type="text" id="name" name="name" required placeholder="例如：技术文章">
        </div>
        <div class="form-group">
            <label for="description">分类描述</label>
            <textarea id="description" name="description" placeholder="输入分类的简要描述"></textarea>
        </div>
        <div style="display: flex; gap: 15px;">
            <button type="submit" class="btn">创建分类</button>
            <a href="/admin/categories" class="btn" style="background: #95a5a6;">取消</a>
        </div>
    </form>
</div>

<script>
document.getElementById('uploadCategoryForm').addEventListener('submit', async function(e) {
    e.preventDefault();

    var formData = new FormData(this);
    var resultDiv = document.getElementById('uploadResult');
    var submitBtn = this.querySelector('button[type="submit"]');

    submitBtn.disabled = true;
    submitBtn.textContent = '上传中...';
    resultDiv.style.display = 'none';

    try {
        var response = await fetch('/api/admin/upload/category', {
            method: 'POST',
            body: formData
        });
        var data = await response.json();

        resultDiv.style.display = 'block';
        if (response.ok) {
            resultDiv.innerHTML = '<div style="color: #155724; padding: 10px; background: #d4edda; border-radius: 6px;">' + data.message + '</div>';
            setTimeout(function() {
                window.location.href = '/admin/categories';
            }, 1500);
        } else {
            resultDiv.innerHTML = '<div style="color: #721c24; padding: 10px; background: #f8d7da; border-radius: 6px;">错误: ' + data.message + '</div>';
        }
    } catch (error) {
        resultDiv.style.display = 'block';
        resultDiv.innerHTML = '<div style="color: #721c24; padding: 10px; background: #f8d7da; border-radius: 6px;">请求失败: ' + error.message + '</div>';
    }

    submitBtn.disabled = false;
    submitBtn.textContent = '上传并创建';
});
</script>
{% endblock content %}"#,
        )?;

        // Admin items template
        tera.add_raw_template(
            "admin_items.html",
            r#"{% extends "admin_base.html" %}
{% block title %}文章管理{% endblock %}
{% block content %}
{% if message %}
<div class="message message-{% if success %}success{% else %}error{% endif %}">{{ message }}</div>
{% endif %}

<div class="card">
    <h2 style="display: flex; justify-content: space-between; align-items: center;">
        文章列表
        <a href="/admin/items/new" class="btn">撰写新文章</a>
    </h2>
    <div class="tabs">
        <a href="/admin/items" class="{% if show_drafts != 'true' %}active{% endif %}">已发布</a>
        <a href="/admin/items?drafts=true" class="{% if show_drafts == 'true' %}active{% endif %}">草稿箱</a>
    </div>
    {% if items | length == 0 %}
    <p style="color: #888; text-align: center; padding: 40px;">
        {% if show_drafts == 'true' %}
        暂无草稿
        {% else %}
        暂无文章
        {% endif %}
    </p>
    {% else %}
    <table class="table">
        <thead>
            <tr>
                <th>标题</th>
                <th>分类</th>
                <th>状态</th>
                <th>操作</th>
            </tr>
        </thead>
        <tbody>
            {% for item in items %}
            <tr>
                <td>{{ item.title }}</td>
                <td>{{ item.category }}</td>
                <td>
                    {% if item.is_draft %}
                    <span class="badge badge-warning">草稿</span>
                    {% else %}
                    <span class="badge badge-success">已发布</span>
                    {% endif %}
                </td>
                <td>
                    <div class="action-btns">
                        <a href="/{{ item.slug }}" class="btn btn-sm" target="_blank">查看</a>
                        <a href="/admin/items/{{ item.slug }}/edit" class="btn btn-sm">编辑</a>
                        {% if item.is_draft %}
                        <form action="/admin/items/{{ item.slug }}/publish" method="POST" style="display: inline;">
                            <button type="submit" class="btn btn-sm btn-success">发布</button>
                        </form>
                        {% endif %}
                        <form action="/admin/items/{{ item.slug }}/delete" method="POST" style="display: inline;" onsubmit="return confirm('确定要删除这篇文章吗？')">
                            <button type="submit" class="btn btn-sm btn-danger">删除</button>
                        </form>
                    </div>
                </td>
            </tr>
            {% endfor %}
        </tbody>
    </table>
    {% endif %}
</div>
{% endblock content %}"#,
        )?;

        // Admin new item template
        tera.add_raw_template(
            "admin_new_item.html",
            r#"{% extends "admin_base.html" %}
{% block title %}撰写新文章{% endblock %}
{% block content %}
<div class="card">
    <h2>撰写新文章</h2>

    <!-- 上传压缩包表单 -->
    <div style="background: #f8f9fa; padding: 20px; border-radius: 8px; margin-bottom: 20px;">
        <h3 style="margin-bottom: 15px;">方式一：上传压缩包（推荐）</h3>
        <p style="color: #666; margin-bottom: 15px;">
            上传一个 .zip 或 .tar.gz 压缩包，压缩包内应包含文章目录，目录中应包含 .md 文件。
        </p>
        <form id="uploadItemForm" enctype="multipart/form-data">
            <div class="grid-2">
                <div class="form-group">
                    <label for="item_category">选择分类</label>
                    <select id="item_category" name="category" required>
                        <option value="">选择分类</option>
                        {% for category in categories %}
                        <option value="{{ category.slug }}">{{ category.title }}</option>
                        {% endfor %}
                    </select>
                </div>
                <div class="form-group">
                    <label for="item_file">选择压缩包</label>
                    <input type="file" id="item_file" name="file" accept=".zip,.tar.gz,.tgz" required>
                </div>
            </div>
            <button type="submit" class="btn">上传并创建</button>
        </form>
        <div id="uploadItemResult" style="margin-top: 15px; display: none;"></div>
    </div>

    <div style="text-align: center; margin: 20px 0; color: #999;">— 或者 —</div>

    <!-- 手动创建表单 -->
    <h3 style="margin-bottom: 15px;">方式二：手动创建</h3>
    <form method="POST" action="/admin/items">
        <div class="grid-2">
            <div class="form-group">
                <label for="category">分类</label>
                <select id="category" name="category" required>
                    <option value="">选择分类</option>
                    {% for category in categories %}
                    <option value="{{ category.slug }}">{{ category.title }}</option>
                    {% endfor %}
                </select>
            </div>
            <div class="form-group">
                <label for="item_name">文章目录名</label>
                <input type="text" id="item_name" name="item_name" required placeholder="例如：hello-world">
                <small style="color: #888;">只能包含字母、数字和连字符</small>
            </div>
        </div>
        <div class="form-group">
            <label for="title">文章标题</label>
            <input type="text" id="title" name="title" required placeholder="输入文章标题">
        </div>
        <div class="form-group">
            <label for="author">作者</label>
            <input type="text" id="author" name="author" value="{{ config.site.author }}">
        </div>
        <div class="form-group">
            <label for="description">文章描述</label>
            <textarea id="description" name="description" placeholder="输入文章描述（可选）"></textarea>
        </div>
        <div class="form-group">
            <label for="tags">标签</label>
            <input type="text" id="tags" name="tags" placeholder="用逗号分隔多个标签">
        </div>
        <div class="form-group">
            <label for="date">发布日期</label>
            <input type="date" id="date" name="date">
        </div>
        <div class="form-group">
            <label for="is_draft">
                <input type="checkbox" id="is_draft" name="is_draft" value="true">
                保存为草稿
            </label>
        </div>
        <div class="form-group">
            <label for="content">文章内容 (Markdown)</label>
            <textarea id="content" name="content" placeholder="使用 Markdown 格式编写文章内容"></textarea>
        </div>
        <div style="display: flex; gap: 15px;">
            <button type="submit" class="btn">保存文章</button>
            <a href="/admin/items" class="btn" style="background: #95a5a6;">取消</a>
        </div>
    </form>
</div>

<script>
document.getElementById('uploadItemForm').addEventListener('submit', async function(e) {
    e.preventDefault();

    var formData = new FormData(this);
    var resultDiv = document.getElementById('uploadItemResult');
    var submitBtn = this.querySelector('button[type="submit"]');
    var category = document.getElementById('item_category').value;

    if (!category) {
        resultDiv.style.display = 'block';
        resultDiv.innerHTML = '<div style="color: #721c24; padding: 10px; background: #f8d7da; border-radius: 6px;">请先选择分类</div>';
        return;
    }

    submitBtn.disabled = true;
    submitBtn.textContent = '上传中...';
    resultDiv.style.display = 'none';

    try {
        var response = await fetch('/api/admin/upload/item/' + category, {
            method: 'POST',
            body: formData
        });
        var data = await response.json();

        resultDiv.style.display = 'block';
        if (response.ok) {
            resultDiv.innerHTML = '<div style="color: #155724; padding: 10px; background: #d4edda; border-radius: 6px;">' + data.message + '</div>';
            setTimeout(function() {
                window.location.href = '/admin/items';
            }, 1500);
        } else {
            resultDiv.innerHTML = '<div style="color: #721c24; padding: 10px; background: #f8d7da; border-radius: 6px;">错误: ' + data.message + '</div>';
        }
    } catch (error) {
        resultDiv.style.display = 'block';
        resultDiv.innerHTML = '<div style="color: #721c24; padding: 10px; background: #f8d7da; border-radius: 6px;">请求失败: ' + error.message + '</div>';
    }

    submitBtn.disabled = false;
    submitBtn.textContent = '上传并创建';
});
</script>
{% endblock content %}"#,
        )?;

        // Admin edit item template
        tera.add_raw_template(
            "admin_edit_item.html",
            r#"{% extends "admin_base.html" %}
{% block title %}编辑文章{% endblock %}
{% block content %}
<div class="card">
    <h2>编辑文章</h2>
    <form method="POST" action="/admin/items/{{ item.slug }}">
        <input type="hidden" name="_method" value="PUT">
        <div class="grid-2">
            <div class="form-group">
                <label for="category">分类</label>
                <select id="category" name="category" required>
                    {% for category in categories %}
                    <option value="{{ category.slug }}" {% if category.slug == item.category %}selected{% endif %}>{{ category.title }}</option>
                    {% endfor %}
                </select>
            </div>
            <div class="form-group">
                <label for="item_name">文章目录名</label>
                <input type="text" id="item_name" name="item_name" required value="{{ item.item_name }}" placeholder="例如：hello-world">
            </div>
        </div>
        <div class="form-group">
            <label for="title">文章标题</label>
            <input type="text" id="title" name="title" required value="{{ item.title }}">
        </div>
        <div class="form-group">
            <label for="author">作者</label>
            <input type="text" id="author" name="author" value="{{ item.author | default(value=config.site.author) }}">
        </div>
        <div class="form-group">
            <label for="description">文章描述</label>
            <textarea id="description" name="description">{{ item.description | default(value="") }}</textarea>
        </div>
        <div class="form-group">
            <label for="tags">标签</label>
            <input type="text" id="tags" name="tags" value="{{ item.tags | default(value="") }}" placeholder="用逗号分隔多个标签">
        </div>
        <div class="form-group">
            <label for="date">发布日期</label>
            <input type="date" id="date" name="date" value="{{ item.date | default(value="") }}">
        </div>
        <div class="form-group">
            <label for="is_draft">
                <input type="checkbox" id="is_draft" name="is_draft" value="true" {% if item.is_draft %}checked{% endif %}>
                保存为草稿
            </label>
        </div>
        <div class="form-group">
            <label for="content">文章内容 (Markdown)</label>
            <textarea id="content" name="content">{{ item.content }}</textarea>
        </div>
        <div style="display: flex; gap: 15px;">
            <button type="submit" class="btn">保存修改</button>
            <a href="/admin/items" class="btn" style="background: #95a5a6;">取消</a>
        </div>
    </form>
</div>
{% endblock content %}"#,
        )?;

        // Admin analytics template
        tera.add_raw_template(
            "admin_analytics.html",
            r#"{% extends "admin_base.html" %}
{% block title %}访问统计{% endblock %}
{% block content %}
{% if message %}
<div class="message message-{% if success %}success{% else %}error{% endif %}">{{ message }}</div>
{% endif %}

<div class="card">
    <h2>访问概览</h2>
    <div class="stats-grid">
        <div class="stat-card">
            <h3 id="totalVisits">-</h3>
            <p>总访问量</p>
        </div>
        <div class="stat-card">
            <h3 id="uniqueVisitors">-</h3>
            <p>独立访客</p>
        </div>
    </div>
</div>

<div class="grid-2">
    <div class="card">
        <h2>访问来源</h2>
        <div id="sourceStats">
            <p style="color: #888; text-align: center; padding: 20px;">加载中...</p>
        </div>
    </div>

    <div class="card">
        <h2>热门页面</h2>
        <div id="popularPages">
            <p style="color: #888; text-align: center; padding: 20px;">加载中...</p>
        </div>
    </div>
</div>

<div class="card">
    <h2>搜索关键词</h2>
    <div id="searchKeywords">
        <p style="color: #888; text-align: center; padding: 20px;">加载中...</p>
    </div>
</div>

<div class="card">
    <h2>每日访问统计</h2>
    <div style="margin-bottom: 15px;">
        <label for="dateRange" style="margin-right: 10px;">时间范围：</label>
        <select id="dateRange" onchange="loadDailyStats()">
            <option value="7">最近7天</option>
            <option value="30" selected>最近30天</option>
            <option value="90">最近90天</option>
        </select>
    </div>
    <div id="dailyStats">
        <p style="color: #888; text-align: center; padding: 20px;">加载中...</p>
    </div>
</div>

<script>
async function loadTotalStats() {
    try {
        const response = await fetch('/api/analytics/total-stats');
        const data = await response.json();
        
        if (data.status === 'success') {
            document.getElementById('totalVisits').textContent = data.stats.total_visits;
            document.getElementById('uniqueVisitors').textContent = data.stats.unique_visitors;
        }
    } catch (error) {
        console.error('加载总统计失败:', error);
    }
}

async function loadSourceStats() {
    try {
        const response = await fetch('/api/analytics/source-stats?limit=10');
        const data = await response.json();
        
        if (data.status === 'success' && data.sources.length > 0) {
            const html = `
                <table class="table">
                    <thead>
                        <tr>
                            <th>来源类型</th>
                            <th>访问量</th>
                            <th>占比</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${data.sources.map(source => `
                            <tr>
                                <td>${source.source}</td>
                                <td>${source.count}</td>
                                <td>${source.percentage.toFixed(1)}%</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
            document.getElementById('sourceStats').innerHTML = html;
        } else {
            document.getElementById('sourceStats').innerHTML = '<p style="color: #888; text-align: center; padding: 20px;">暂无数据</p>';
        }
    } catch (error) {
        console.error('加载来源统计失败:', error);
        document.getElementById('sourceStats').innerHTML = '<p style="color: #c00; text-align: center; padding: 20px;">加载失败</p>';
    }
}

async function loadPopularPages() {
    try {
        const response = await fetch('/api/analytics/popular-pages?limit=10');
        const data = await response.json();
        
        if (data.status === 'success' && data.pages.length > 0) {
            const html = `
                <table class="table">
                    <thead>
                        <tr>
                            <th>页面</th>
                            <th>访问量</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${data.pages.map(page => `
                            <tr>
                                <td><a href="/${page.slug}" target="_blank">${page.title || page.slug}</a></td>
                                <td>${page.visits}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
            document.getElementById('popularPages').innerHTML = html;
        } else {
            document.getElementById('popularPages').innerHTML = '<p style="color: #888; text-align: center; padding: 20px;">暂无数据</p>';
        }
    } catch (error) {
        console.error('加载热门页面失败:', error);
        document.getElementById('popularPages').innerHTML = '<p style="color: #c00; text-align: center; padding: 20px;">加载失败</p>';
    }
}

async function loadSearchKeywords() {
    try {
        const response = await fetch('/api/analytics/search-keywords?limit=15');
        const data = await response.json();
        
        if (data.status === 'success' && data.keywords.length > 0) {
            const html = `
                <table class="table">
                    <thead>
                        <tr>
                            <th>关键词</th>
                            <th>搜索次数</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${data.keywords.map(([keyword, count]) => `
                            <tr>
                                <td>${keyword}</td>
                                <td>${count}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
            document.getElementById('searchKeywords').innerHTML = html;
        } else {
            document.getElementById('searchKeywords').innerHTML = '<p style="color: #888; text-align: center; padding: 20px;">暂无搜索数据</p>';
        }
    } catch (error) {
        console.error('加载搜索关键词失败:', error);
        document.getElementById('searchKeywords').innerHTML = '<p style="color: #c00; text-align: center; padding: 20px;">加载失败</p>';
    }
}

async function loadDailyStats() {
    const days = document.getElementById('dateRange').value;
    let startDate = new Date();
    startDate.setDate(startDate.getDate() - parseInt(days));
    const startDateStr = startDate.toISOString().split('T')[0];
    
    try {
        const response = await fetch(`/api/analytics/daily-stats?start_date=${startDateStr}`);
        const data = await response.json();
        
        if (data.status === 'success' && data.stats.length > 0) {
            const html = `
                <table class="table">
                    <thead>
                        <tr>
                            <th>日期</th>
                            <th>总访问</th>
                            <th>独立访客</th>
                            <th>页面浏览</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${data.stats.map(stat => `
                            <tr>
                                <td>${stat.date}</td>
                                <td>${stat.total_visits}</td>
                                <td>${stat.unique_visitors}</td>
                                <td>${stat.page_views}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
            document.getElementById('dailyStats').innerHTML = html;
        } else {
            document.getElementById('dailyStats').innerHTML = '<p style="color: #888; text-align: center; padding: 20px;">暂无数据</p>';
        }
    } catch (error) {
        console.error('加载每日统计失败:', error);
        document.getElementById('dailyStats').innerHTML = '<p style="color: #c00; text-align: center; padding: 20px;">加载失败</p>';
    }
}

// 页面加载时初始化所有数据
loadTotalStats();
loadSourceStats();
loadPopularPages();
loadSearchKeywords();
loadDailyStats();
</script>
{% endblock content %}"#,
        )?;

        // Admin compile template
        tera.add_raw_template(
            "admin_compile.html",
            r#"{% extends "admin_base.html" %}
{% block title %}编译发布{% endblock %}
{% block content %}
{% if message %}
<div class="message message-{% if success %}success{% else %}error{% endif %}">{{ message }}</div>
{% endif %}

<div class="card">
    <h2>编译并发布</h2>
    <p style="margin-bottom: 20px; color: #666;">
        点击下方按钮将重新编译所有内容并更新站点。编译过程可能需要几秒钟时间。
    </p>
    <form method="POST" action="/api/recompile" id="compileForm">
        <button type="submit" class="btn btn-success" id="compileBtn">
            开始编译
        </button>
    </form>
    <div id="compileResult" style="margin-top: 20px; display: none;">
        <div class="card" style="background: #f8f9fa;">
            <h3>编译结果</h3>
            <pre id="resultContent" style="white-space: pre-wrap; word-wrap: break-word;"></pre>
        </div>
    </div>
</div>

<script>
    document.getElementById('compileForm').addEventListener('submit', async function(e) {
        e.preventDefault();
        var btn = document.getElementById('compileBtn');
        var resultDiv = document.getElementById('compileResult');
        var resultContent = document.getElementById('resultContent');
        
        btn.disabled = true;
        btn.textContent = '编译中...';
        resultDiv.style.display = 'none';
        
        try {
            var response = await fetch('/api/recompile', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                }
            });
            var data = await response.json();
            
            resultDiv.style.display = 'block';
            if (response.ok) {
                resultContent.textContent = '编译成功！\n\n' +
                    '分类数量: ' + data.categories + '\n' +
                    '文章数量: ' + data.items + '\n' +
                    '附件数量: ' + data.attachments + '\n\n' +
                    data.message;
                resultContent.style.color = '#155724';
            } else {
                resultContent.textContent = '编译失败:\n' + data.message;
                resultContent.style.color = '#721c24';
            }
        } catch (error) {
            resultDiv.style.display = 'block';
            resultContent.textContent = '请求失败: ' + error.message;
            resultContent.style.color = '#721c24';
        }
        
        btn.disabled = false;
        btn.textContent = '开始编译';
    });
</script>
{% endblock content %}"#,
        )?;

        Ok(tera)
    }

    pub fn render_index(&self, site_content: &SiteContent) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("site_content", site_content);
        Ok(self.tera.render("index.html", &context)?)
    }

    pub fn render_category(&self, category: &Category) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("category", category);
        Ok(self.tera.render("category.html", &context)?)
    }

    pub fn render_item(&self, item: &ContentItem) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("item", item);
        Ok(self.tera.render("item.html", &context)?)
    }

    pub fn render_archive(&self, all_items: &Vec<(String, String, String, Option<String>)>, total_items: usize) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("all_items", all_items);
        context.insert("total_items", &total_items);
        Ok(self.tera.render("archive.html", &context)?)
    }

    pub fn render_admin_login(&self, error: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        if let Some(err) = error {
            context.insert("error", err);
        }
        Ok(self.tera.render("admin_login.html", &context)?)
    }

    pub fn render_admin_base(&self, active: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("active", active);
        Ok(self.tera.render("admin_base.html", &context)?)
    }

    pub fn render_admin_overview(&self, overview: &serde_json::Value, last_compiled: Option<&str>, message: Option<&str>, success: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("overview", overview);
        context.insert("last_compiled", &last_compiled);
        if let Some(msg) = message {
            context.insert("message", msg);
            context.insert("success", &success);
        }
        context.insert("active", "overview");
        Ok(self.tera.render("admin_overview.html", &context)?)
    }

    pub fn render_admin_categories(&self, categories: &[serde_json::Value], message: Option<&str>, success: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("categories", categories);
        if let Some(msg) = message {
            context.insert("message", msg);
            context.insert("success", &success);
        }
        context.insert("active", "categories");
        Ok(self.tera.render("admin_categories.html", &context)?)
    }

    pub fn render_admin_new_category(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("active", "categories");
        Ok(self.tera.render("admin_new_category.html", &context)?)
    }

    pub fn render_admin_items(&self, items: &[serde_json::Value], show_drafts: bool, message: Option<&str>, success: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("items", items);
        context.insert("show_drafts", &show_drafts.to_string());
        if let Some(msg) = message {
            context.insert("message", msg);
            context.insert("success", &success);
        }
        context.insert("active", "items");
        Ok(self.tera.render("admin_items.html", &context)?)
    }

    pub fn render_admin_new_item(&self, categories: &[serde_json::Value]) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("categories", categories);
        context.insert("active", "items");
        Ok(self.tera.render("admin_new_item.html", &context)?)
    }

    pub fn render_admin_edit_item(&self, item: &serde_json::Value, categories: &[serde_json::Value]) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        context.insert("item", item);
        context.insert("categories", categories);
        context.insert("active", "items");
        Ok(self.tera.render("admin_edit_item.html", &context)?)
    }

    pub fn render_admin_analytics(&self, message: Option<&str>, success: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        if let Some(msg) = message {
            context.insert("message", msg);
            context.insert("success", &success);
        }
        context.insert("active", "analytics");
        Ok(self.tera.render("admin_analytics.html", &context)?)
    }

    pub fn render_admin_compile(&self, message: Option<&str>, success: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut context = Context::new();
        context.insert("config", &self.config);
        if let Some(msg) = message {
            context.insert("message", msg);
            context.insert("success", &success);
        }
        context.insert("active", "compile");
        Ok(self.tera.render("admin_compile.html", &context)?)
    }

    pub fn render_with_context(&self, template: &str, context: &Context) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.tera.render(template, context)?)
    }
}