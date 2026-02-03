use ratatui::style::Color;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Theme {
    // Border colors
    pub border: String,
    pub border_active: String,

    // Selection
    pub selected_bg: String,

    // Status colors
    pub status_working: String,
    pub status_active: String,
    pub status_idle: String,
    pub status_inactive: String,
    pub status_waiting: String,

    // Diff colors
    pub diff_add: String,
    pub diff_remove: String,
    pub diff_hunk: String,

    // General
    pub text: String,
    pub text_muted: String,
    pub highlight: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border: "#5c6370".to_string(),
            border_active: "#98c379".to_string(),
            selected_bg: "#1e3250".to_string(),
            status_working: "#56b6c2".to_string(),
            status_active: "#98c379".to_string(),
            status_idle: "#e5c07b".to_string(),
            status_inactive: "#5c6370".to_string(),
            status_waiting: "#c678dd".to_string(),
            diff_add: "#98c379".to_string(),
            diff_remove: "#e06c75".to_string(),
            diff_hunk: "#61afef".to_string(),
            text: "#abb2bf".to_string(),
            text_muted: "#5c6370".to_string(),
            highlight: "#61afef".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let paths = [
            dirs::config_dir().map(|p| p.join("lazychat/config.toml")),
            dirs::home_dir().map(|p| p.join(".lazychat.toml")),
            Some(PathBuf::from("lazychat.toml")),
        ];

        for path in paths.into_iter().flatten() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }

        Config::default()
    }
}

impl Theme {
    pub fn parse_color(&self, hex: &str) -> Color {
        parse_hex_color(hex).unwrap_or(Color::White)
    }

    pub fn border(&self) -> Color {
        self.parse_color(&self.border)
    }

    pub fn border_active(&self) -> Color {
        self.parse_color(&self.border_active)
    }

    pub fn selected_bg(&self) -> Color {
        self.parse_color(&self.selected_bg)
    }

    pub fn status_working(&self) -> Color {
        self.parse_color(&self.status_working)
    }

    pub fn status_active(&self) -> Color {
        self.parse_color(&self.status_active)
    }

    pub fn status_idle(&self) -> Color {
        self.parse_color(&self.status_idle)
    }

    pub fn status_inactive(&self) -> Color {
        self.parse_color(&self.status_inactive)
    }

    pub fn status_waiting(&self) -> Color {
        self.parse_color(&self.status_waiting)
    }

    pub fn diff_add(&self) -> Color {
        self.parse_color(&self.diff_add)
    }

    pub fn diff_remove(&self) -> Color {
        self.parse_color(&self.diff_remove)
    }

    pub fn diff_hunk(&self) -> Color {
        self.parse_color(&self.diff_hunk)
    }

    pub fn text(&self) -> Color {
        self.parse_color(&self.text)
    }

    pub fn text_muted(&self) -> Color {
        self.parse_color(&self.text_muted)
    }

    pub fn highlight(&self) -> Color {
        self.parse_color(&self.highlight)
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}
