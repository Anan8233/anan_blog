use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub light_mode: ColorPalette,
    pub dark_mode: ColorPalette,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub background: String,
    pub foreground: String,
    pub primary: String,
    pub secondary: String,
    pub accent: String,
    pub border: String,
    pub muted: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            light_mode: ColorPalette {
                background: "#ffffff".to_string(),
                foreground: "#333333".to_string(),
                primary: "#3498db".to_string(),
                secondary: "#2c3e50".to_string(),
                accent: "#e74c3c".to_string(),
                border: "#e0e0e0".to_string(),
                muted: "#666666".to_string(),
            },
            dark_mode: ColorPalette {
                background: "#1a1a1a".to_string(),
                foreground: "#e0e0e0".to_string(),
                primary: "#5dade2".to_string(),
                secondary: "#34495e".to_string(),
                accent: "#ec7063".to_string(),
                border: "#333333".to_string(),
                muted: "#999999".to_string(),
            },
        }
    }
}

impl Theme {
    pub fn to_css(&self) -> String {
        format!(
            r#"
:root {{
    --bg-color: {};
    --fg-color: {};
    --primary-color: {};
    --secondary-color: {};
    --accent-color: {};
    --border-color: {};
    --muted-color: {};
}}

@media (prefers-color-scheme: dark) {{
    :root {{
        --bg-color: {};
        --fg-color: {};
        --primary-color: {};
        --secondary-color: {};
        --accent-color: {};
        --border-color: {};
        --muted-color: {};
    }}
}}

body {{
    background-color: var(--bg-color);
    color: var(--fg-color);
}}
"#,
            self.light_mode.background,
            self.light_mode.foreground,
            self.light_mode.primary,
            self.light_mode.secondary,
            self.light_mode.accent,
            self.light_mode.border,
            self.light_mode.muted,
            self.dark_mode.background,
            self.dark_mode.foreground,
            self.dark_mode.primary,
            self.dark_mode.secondary,
            self.dark_mode.accent,
            self.dark_mode.border,
            self.dark_mode.muted,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme.name, "default");
    }

    #[test]
    fn test_theme_to_css() {
        let theme = Theme::default();
        let css = theme.to_css();
        assert!(css.contains("var(--bg-color)"));
        assert!(css.contains("var(--fg-color)"));
    }
}
