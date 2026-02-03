//! Preset configuration for project templates

use anyhow::{Context, Result};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A project preset defining Claude instance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// Unique name for the preset
    pub name: String,
    /// Short keyboard shortcut (e.g., "enb" for energyboard)
    pub shortcut: Option<String>,
    /// Working directory (supports ~ expansion)
    pub cwd: String,
    /// Additional directories to include
    #[serde(default)]
    pub add_dirs: Vec<String>,
    /// Number of Claude instances to spawn
    #[serde(default = "default_instances")]
    pub instances: u32,
    /// Extra CLI arguments for Claude
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_instances() -> u32 {
    1
}

/// Configuration file structure
#[derive(Debug, Default, Serialize, Deserialize)]
struct PresetConfig {
    #[serde(default)]
    preset: Vec<Preset>,
}

/// Manager for loading and querying presets
pub struct PresetManager {
    presets: Vec<Preset>,
    config_path: PathBuf,
    matcher: SkimMatcherV2,
}

impl PresetManager {
    /// Load presets from ~/.config/lazychat/presets.toml
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create default config if it doesn't exist
        if !config_path.exists() {
            Self::create_default_config(&config_path)?;
        }

        // Load and parse config
        let content = fs::read_to_string(&config_path).context("Failed to read presets.toml")?;

        let config: PresetConfig =
            toml::from_str(&content).context("Failed to parse presets.toml")?;

        // Expand ~ in paths
        let presets: Vec<Preset> = config
            .preset
            .into_iter()
            .map(|mut p| {
                p.cwd = expand_tilde(&p.cwd);
                p.add_dirs = p.add_dirs.into_iter().map(|d| expand_tilde(&d)).collect();
                p
            })
            .collect();

        Ok(Self {
            presets,
            config_path,
            matcher: SkimMatcherV2::default(),
        })
    }

    /// Get the config file path
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("lazychat")
            .join("presets.toml")
    }

    /// Create default configuration file
    fn create_default_config(path: &PathBuf) -> Result<()> {
        let default_config = r#"# Lazychat Presets Configuration
# Define project presets for quick Claude instance spawning

# Example preset:
# [[preset]]
# name = "myproject"
# shortcut = "mp"
# cwd = "~/dev/myproject"
# add_dirs = ["../shared-lib"]
# instances = 2
# extra_args = ["--dangerously-skip-permissions"]

[[preset]]
name = "lazychat"
shortcut = "lc"
cwd = "~/dev/lazychat"
instances = 1
extra_args = ["--dangerously-skip-permissions"]
"#;

        fs::write(path, default_config)?;
        Ok(())
    }

    /// Get all presets
    pub fn all(&self) -> &[Preset] {
        &self.presets
    }

    /// Find preset by exact name
    pub fn find_by_name(&self, name: &str) -> Option<&Preset> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// Find preset by shortcut
    pub fn find_by_shortcut(&self, shortcut: &str) -> Option<&Preset> {
        self.presets
            .iter()
            .find(|p| p.shortcut.as_ref().map(|s| s == shortcut).unwrap_or(false))
    }

    /// Fuzzy search presets by query (matches name and shortcut)
    pub fn fuzzy_search(&self, query: &str) -> Vec<(&Preset, i64)> {
        if query.is_empty() {
            return self.presets.iter().map(|p| (p, 0i64)).collect();
        }

        let mut results: Vec<(&Preset, i64)> = self
            .presets
            .iter()
            .filter_map(|preset| {
                // Match against name
                let name_score = self.matcher.fuzzy_match(&preset.name, query);

                // Match against shortcut if present
                let shortcut_score = preset
                    .shortcut
                    .as_ref()
                    .and_then(|s| self.matcher.fuzzy_match(s, query));

                // Take the best score
                let best_score = match (name_score, shortcut_score) {
                    (Some(n), Some(s)) => Some(n.max(s)),
                    (Some(n), None) => Some(n),
                    (None, Some(s)) => Some(s),
                    (None, None) => None,
                };

                best_score.map(|score| (preset, score))
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

    /// Reload configuration from disk
    pub fn reload(&mut self) -> Result<()> {
        let content =
            fs::read_to_string(&self.config_path).context("Failed to read presets.toml")?;

        let config: PresetConfig =
            toml::from_str(&content).context("Failed to parse presets.toml")?;

        self.presets = config
            .preset
            .into_iter()
            .map(|mut p| {
                p.cwd = expand_tilde(&p.cwd);
                p.add_dirs = p.add_dirs.into_iter().map(|d| expand_tilde(&d)).collect();
                p
            })
            .collect();

        Ok(())
    }

    /// Get the config file path (for display/editing)
    pub fn get_config_path(&self) -> &PathBuf {
        &self.config_path
    }
}

/// Expand ~ to home directory in paths
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    path.to_string()
}
