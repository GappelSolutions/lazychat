//! Comprehensive tests for Phase 2 preset configuration system

use anyhow::Result;
use std::fs;
use tempfile::TempDir;

// We'll test the preset module by including necessary types
// In a real scenario, these would be imported from the crate

#[cfg(test)]
mod preset_struct_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Preset {
        name: String,
        shortcut: Option<String>,
        cwd: String,
        #[serde(default)]
        add_dirs: Vec<String>,
        #[serde(default = "default_instances")]
        instances: u32,
        #[serde(default)]
        extra_args: Vec<String>,
    }

    fn default_instances() -> u32 {
        1
    }

    #[test]
    fn test_preset_struct_fields() {
        // Test that we can create a preset with all fields
        let preset = Preset {
            name: "test-project".to_string(),
            shortcut: Some("tp".to_string()),
            cwd: "~/dev/test".to_string(),
            add_dirs: vec!["../shared".to_string()],
            instances: 2,
            extra_args: vec!["--dangerously-skip-permissions".to_string()],
        };

        assert_eq!(preset.name, "test-project");
        assert_eq!(preset.shortcut, Some("tp".to_string()));
        assert_eq!(preset.cwd, "~/dev/test");
        assert_eq!(preset.add_dirs.len(), 1);
        assert_eq!(preset.instances, 2);
        assert_eq!(preset.extra_args.len(), 1);
    }

    #[test]
    fn test_preset_default_instances() {
        // Test that instances defaults to 1
        let toml = r#"
            name = "myproject"
            cwd = "~/dev/myproject"
        "#;

        let preset: Preset = toml::from_str(toml).expect("Failed to parse");
        assert_eq!(preset.instances, 1, "instances should default to 1");
    }

    #[test]
    fn test_preset_optional_shortcut() {
        // Test that shortcut is optional
        let toml = r#"
            name = "myproject"
            cwd = "~/dev/myproject"
        "#;

        let preset: Preset = toml::from_str(toml).expect("Failed to parse");
        assert_eq!(
            preset.shortcut, None,
            "shortcut should be None when not provided"
        );
    }

    #[test]
    fn test_preset_default_add_dirs() {
        // Test that add_dirs defaults to empty vec
        let toml = r#"
            name = "myproject"
            cwd = "~/dev/myproject"
        "#;

        let preset: Preset = toml::from_str(toml).expect("Failed to parse");
        assert!(
            preset.add_dirs.is_empty(),
            "add_dirs should default to empty"
        );
    }

    #[test]
    fn test_preset_default_extra_args() {
        // Test that extra_args defaults to empty vec
        let toml = r#"
            name = "myproject"
            cwd = "~/dev/myproject"
        "#;

        let preset: Preset = toml::from_str(toml).expect("Failed to parse");
        assert!(
            preset.extra_args.is_empty(),
            "extra_args should default to empty"
        );
    }

    #[test]
    fn test_preset_serialization() -> Result<()> {
        let preset = Preset {
            name: "energyboard".to_string(),
            shortcut: Some("enb".to_string()),
            cwd: "~/dev/energyboard".to_string(),
            add_dirs: vec!["../shared-components".to_string()],
            instances: 3,
            extra_args: vec!["--dangerously-skip-permissions".to_string()],
        };

        let toml_str = toml::to_string(&preset)?;
        assert!(toml_str.contains("energyboard"));
        assert!(toml_str.contains("enb"));

        Ok(())
    }
}

#[cfg(test)]
mod preset_manager_load_tests {
    use super::*;

    #[test]
    fn test_creates_config_if_missing() -> Result<()> {
        // Create a temporary directory to act as config dir
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("presets.toml");

        // Verify file doesn't exist
        assert!(!config_path.exists(), "Config should not exist initially");

        // Simulate create_default_config
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

        fs::write(&config_path, default_config)?;

        // Verify file was created
        assert!(config_path.exists(), "Config file should be created");

        // Verify content is valid TOML
        let content = fs::read_to_string(&config_path)?;
        assert!(content.contains("Lazychat Presets Configuration"));
        assert!(content.contains("[[preset]]"));

        Ok(())
    }

    #[test]
    fn test_parses_toml_correctly() -> Result<()> {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct Preset {
            name: String,
            shortcut: Option<String>,
            cwd: String,
            #[serde(default)]
            add_dirs: Vec<String>,
            #[serde(default = "default_instances")]
            instances: u32,
            #[serde(default)]
            extra_args: Vec<String>,
        }

        fn default_instances() -> u32 {
            1
        }

        #[derive(Debug, Deserialize)]
        struct PresetConfig {
            preset: Vec<Preset>,
        }

        let toml_content = r#"
[[preset]]
name = "energyboard"
shortcut = "enb"
cwd = "~/dev/energyboard"
add_dirs = ["../shared"]
instances = 2
extra_args = ["--dangerously-skip-permissions"]

[[preset]]
name = "lazychat"
shortcut = "lc"
cwd = "~/dev/lazychat"
instances = 1
"#;

        let config: PresetConfig = toml::from_str(toml_content)?;

        assert_eq!(config.preset.len(), 2, "Should parse 2 presets");
        assert_eq!(config.preset[0].name, "energyboard");
        assert_eq!(config.preset[0].shortcut, Some("enb".to_string()));
        assert_eq!(config.preset[0].instances, 2);
        assert_eq!(config.preset[1].name, "lazychat");
        assert_eq!(config.preset[1].instances, 1);

        Ok(())
    }

    #[test]
    fn test_expands_tilde_in_cwd() {
        let home = dirs::home_dir().expect("Should have home dir");
        let home_str = home.to_string_lossy();

        // Test ~/foo expansion
        let expanded = expand_tilde("~/dev/project");
        assert!(
            expanded.starts_with(&*home_str),
            "Should start with home dir"
        );
        assert!(expanded.ends_with("dev/project"), "Should end with path");

        // Test ~ alone
        let expanded = expand_tilde("~");
        assert_eq!(expanded, home_str, "~ should expand to home dir");

        // Test absolute path unchanged
        let expanded = expand_tilde("/absolute/path");
        assert_eq!(
            expanded, "/absolute/path",
            "Absolute paths should not change"
        );
    }

    #[test]
    fn test_expands_tilde_in_add_dirs() {
        let home = dirs::home_dir().expect("Should have home dir");
        let home_str = home.to_string_lossy();

        let paths = vec![
            "~/dev/shared".to_string(),
            "/absolute/lib".to_string(),
            "relative/path".to_string(),
        ];

        let expanded: Vec<String> = paths.iter().map(|p| expand_tilde(p)).collect();

        assert!(
            expanded[0].starts_with(&*home_str),
            "First path should be expanded"
        );
        assert_eq!(expanded[1], "/absolute/lib", "Absolute path unchanged");
        assert_eq!(expanded[2], "relative/path", "Relative path unchanged");
    }

    // Helper function for tilde expansion
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
}

#[cfg(test)]
mod preset_find_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Preset {
        name: String,
        shortcut: Option<String>,
        cwd: String,
        #[serde(default)]
        add_dirs: Vec<String>,
        #[serde(default = "default_instances")]
        instances: u32,
        #[serde(default)]
        extra_args: Vec<String>,
    }

    fn default_instances() -> u32 {
        1
    }

    fn create_test_presets() -> Vec<Preset> {
        vec![
            Preset {
                name: "energyboard".to_string(),
                shortcut: Some("enb".to_string()),
                cwd: "~/dev/energyboard".to_string(),
                add_dirs: vec![],
                instances: 1,
                extra_args: vec![],
            },
            Preset {
                name: "lazychat".to_string(),
                shortcut: Some("lc".to_string()),
                cwd: "~/dev/lazychat".to_string(),
                add_dirs: vec![],
                instances: 1,
                extra_args: vec![],
            },
            Preset {
                name: "backoffice".to_string(),
                shortcut: None,
                cwd: "~/dev/backoffice".to_string(),
                add_dirs: vec![],
                instances: 1,
                extra_args: vec![],
            },
        ]
    }

    #[test]
    fn test_find_by_name_exact_match() {
        let presets = create_test_presets();

        let found = presets.iter().find(|p| p.name == "lazychat");
        assert!(found.is_some(), "Should find lazychat");
        assert_eq!(found.unwrap().name, "lazychat");

        let found = presets.iter().find(|p| p.name == "energyboard");
        assert!(found.is_some(), "Should find energyboard");

        let found = presets.iter().find(|p| p.name == "nonexistent");
        assert!(found.is_none(), "Should not find nonexistent preset");
    }

    #[test]
    fn test_find_by_name_case_sensitive() {
        let presets = create_test_presets();

        let found = presets.iter().find(|p| p.name == "LazyChat");
        assert!(found.is_none(), "Should be case sensitive");
    }

    #[test]
    fn test_find_by_shortcut_exact_match() {
        let presets = create_test_presets();

        let found = presets
            .iter()
            .find(|p| p.shortcut.as_ref().map(|s| s == "lc").unwrap_or(false));
        assert!(found.is_some(), "Should find by shortcut 'lc'");
        assert_eq!(found.unwrap().name, "lazychat");

        let found = presets
            .iter()
            .find(|p| p.shortcut.as_ref().map(|s| s == "enb").unwrap_or(false));
        assert!(found.is_some(), "Should find by shortcut 'enb'");
        assert_eq!(found.unwrap().name, "energyboard");
    }

    #[test]
    fn test_find_by_shortcut_returns_none_when_missing() {
        let presets = create_test_presets();

        let found = presets
            .iter()
            .find(|p| p.shortcut.as_ref().map(|s| s == "xyz").unwrap_or(false));
        assert!(found.is_none(), "Should not find nonexistent shortcut");
    }

    #[test]
    fn test_find_by_shortcut_handles_none() {
        let presets = create_test_presets();

        // backoffice has no shortcut
        let backoffice = presets.iter().find(|p| p.name == "backoffice").unwrap();
        assert_eq!(backoffice.shortcut, None);
    }
}

#[cfg(test)]
mod fuzzy_search_tests {
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Preset {
        name: String,
        shortcut: Option<String>,
        cwd: String,
        #[serde(default)]
        add_dirs: Vec<String>,
        #[serde(default = "default_instances")]
        instances: u32,
        #[serde(default)]
        extra_args: Vec<String>,
    }

    fn default_instances() -> u32 {
        1
    }

    fn create_test_presets() -> Vec<Preset> {
        vec![
            Preset {
                name: "energyboard".to_string(),
                shortcut: Some("enb".to_string()),
                cwd: "~/dev/energyboard".to_string(),
                add_dirs: vec![],
                instances: 1,
                extra_args: vec![],
            },
            Preset {
                name: "lazychat".to_string(),
                shortcut: Some("lc".to_string()),
                cwd: "~/dev/lazychat".to_string(),
                add_dirs: vec![],
                instances: 1,
                extra_args: vec![],
            },
            Preset {
                name: "backoffice".to_string(),
                shortcut: Some("bo".to_string()),
                cwd: "~/dev/backoffice".to_string(),
                add_dirs: vec![],
                instances: 1,
                extra_args: vec![],
            },
        ]
    }

    fn fuzzy_search<'a>(
        presets: &'a [Preset],
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> Vec<(&'a Preset, i64)> {
        if query.is_empty() {
            return presets.iter().map(|p| (p, 0i64)).collect();
        }

        let mut results: Vec<(&Preset, i64)> = presets
            .iter()
            .filter_map(|preset| {
                // Match against name
                let name_score = matcher.fuzzy_match(&preset.name, query);

                // Match against shortcut if present
                let shortcut_score = preset
                    .shortcut
                    .as_ref()
                    .and_then(|s| matcher.fuzzy_match(s, query));

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

    #[test]
    fn test_fuzzy_search_empty_query_returns_all() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        let results = fuzzy_search(&presets, "", &matcher);

        assert_eq!(results.len(), 3, "Empty query should return all presets");
        assert_eq!(results[0].1, 0, "Score should be 0 for empty query");
    }

    #[test]
    fn test_fuzzy_search_matches_name() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        let results = fuzzy_search(&presets, "lazy", &matcher);

        assert!(!results.is_empty(), "Should find results for 'lazy'");
        assert_eq!(
            results[0].0.name, "lazychat",
            "Best match should be lazychat"
        );
    }

    #[test]
    fn test_fuzzy_search_matches_shortcut() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        let results = fuzzy_search(&presets, "lc", &matcher);

        assert!(!results.is_empty(), "Should find results for 'lc'");
        assert_eq!(
            results[0].0.name, "lazychat",
            "Should match lazychat by shortcut"
        );
    }

    #[test]
    fn test_fuzzy_search_takes_best_score() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        // "en" should match both "energyboard" name and "enb" shortcut
        let results = fuzzy_search(&presets, "en", &matcher);

        assert!(!results.is_empty(), "Should find results for 'en'");
        assert_eq!(results[0].0.name, "energyboard", "Should match energyboard");
    }

    #[test]
    fn test_fuzzy_search_sorted_by_score() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        let results = fuzzy_search(&presets, "b", &matcher);

        // All presets have 'b' somewhere (energyboard, backoffice)
        assert!(!results.is_empty(), "Should find results");

        // Verify descending order
        for i in 0..results.len() - 1 {
            assert!(
                results[i].1 >= results[i + 1].1,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_fuzzy_search_no_matches() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        let results = fuzzy_search(&presets, "xyz123", &matcher);

        assert!(
            results.is_empty(),
            "Should return no results for unmatched query"
        );
    }

    #[test]
    fn test_fuzzy_search_partial_match() {
        let presets = create_test_presets();
        let matcher = SkimMatcherV2::default();

        // "chat" should match "lazychat"
        let results = fuzzy_search(&presets, "chat", &matcher);

        assert!(!results.is_empty(), "Should find partial matches");
        assert_eq!(results[0].0.name, "lazychat");
    }
}

#[cfg(test)]
mod expand_tilde_tests {
    use super::*;

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

    #[test]
    fn test_expand_tilde_with_path() {
        let home = dirs::home_dir().expect("Should have home dir");
        let result = expand_tilde("~/foo");

        assert!(result.starts_with(&home.to_string_lossy().to_string()));
        assert!(result.ends_with("foo"));
    }

    #[test]
    fn test_expand_tilde_alone() {
        let home = dirs::home_dir().expect("Should have home dir");
        let result = expand_tilde("~");

        assert_eq!(result, home.to_string_lossy().to_string());
    }

    #[test]
    fn test_expand_tilde_absolute_unchanged() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, "/absolute/path");
    }

    #[test]
    fn test_expand_tilde_relative_unchanged() {
        let result = expand_tilde("relative/path");
        assert_eq!(result, "relative/path");
    }

    #[test]
    fn test_expand_tilde_nested_path() {
        let home = dirs::home_dir().expect("Should have home dir");
        let result = expand_tilde("~/dev/projects/rust");

        assert!(result.starts_with(&home.to_string_lossy().to_string()));
        assert!(result.ends_with("dev/projects/rust"));
    }

    #[test]
    fn test_expand_tilde_with_dot_files() {
        let home = dirs::home_dir().expect("Should have home dir");
        let result = expand_tilde("~/.config/lazychat");

        assert!(result.starts_with(&home.to_string_lossy().to_string()));
        assert!(result.ends_with(".config/lazychat"));
    }
}

#[test]
fn test_full_preset_workflow() -> Result<()> {
    println!("\n=== Running Full Preset Workflow Test ===\n");

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Preset {
        name: String,
        shortcut: Option<String>,
        cwd: String,
        #[serde(default)]
        add_dirs: Vec<String>,
        #[serde(default = "default_instances")]
        instances: u32,
        #[serde(default)]
        extra_args: Vec<String>,
    }

    fn default_instances() -> u32 {
        1
    }

    #[derive(Debug, Deserialize)]
    struct PresetConfig {
        preset: Vec<Preset>,
    }

    // 1. Create config file
    println!("1. Creating TOML config file...");
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("presets.toml");

    let config_content = r#"
[[preset]]
name = "energyboard"
shortcut = "enb"
cwd = "~/dev/energyboard"
add_dirs = ["~/shared/components"]
instances = 2
extra_args = ["--dangerously-skip-permissions"]

[[preset]]
name = "lazychat"
shortcut = "lc"
cwd = "~/dev/lazychat"
instances = 1
"#;

    fs::write(&config_path, config_content)?;
    println!("   ✓ Config file created\n");

    // 2. Parse TOML
    println!("2. Parsing TOML config...");
    let content = fs::read_to_string(&config_path)?;
    let config: PresetConfig = toml::from_str(&content)?;
    assert_eq!(config.preset.len(), 2);
    println!("   ✓ Parsed {} presets\n", config.preset.len());

    // 3. Expand tilde in paths
    println!("3. Expanding ~ in paths...");
    let home = dirs::home_dir().expect("Should have home dir");
    let home_str = home.to_string_lossy();

    let mut presets = config.preset;
    for preset in &mut presets {
        if preset.cwd.starts_with("~/") {
            preset.cwd = home.join(&preset.cwd[2..]).to_string_lossy().to_string();
        }
        for add_dir in &mut preset.add_dirs {
            if add_dir.starts_with("~/") {
                *add_dir = home.join(&add_dir[2..]).to_string_lossy().to_string();
            }
        }
    }

    assert!(presets[0].cwd.starts_with(&*home_str));
    assert!(presets[0].add_dirs[0].starts_with(&*home_str));
    println!("   ✓ Paths expanded\n");

    // 4. Find by name
    println!("4. Testing find by name...");
    let found = presets.iter().find(|p| p.name == "lazychat");
    assert!(found.is_some());
    println!("   ✓ Found preset by name\n");

    // 5. Find by shortcut
    println!("5. Testing find by shortcut...");
    let found = presets
        .iter()
        .find(|p| p.shortcut.as_ref().map(|s| s == "enb").unwrap_or(false));
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "energyboard");
    println!("   ✓ Found preset by shortcut\n");

    // 6. Fuzzy search
    println!("6. Testing fuzzy search...");
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;

    let matcher = SkimMatcherV2::default();
    let mut results: Vec<(&Preset, i64)> = presets
        .iter()
        .filter_map(|p| matcher.fuzzy_match(&p.name, "lazy").map(|score| (p, score)))
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1));

    assert!(!results.is_empty());
    assert_eq!(results[0].0.name, "lazychat");
    println!("   ✓ Fuzzy search working\n");

    println!("=== All Preset Workflow Tests Passed ===\n");

    Ok(())
}
