// rdump/src/config.rs - FINAL CORRECTED VERSION

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub presets: HashMap<String, String>,
}

/// Returns the path to the global configuration file.
/// It can be overridden by the RDUMP_TEST_CONFIG_DIR environment variable for testing.
pub fn global_config_path() -> Option<PathBuf> {
    // First, check for the override environment variable. This is active in ALL builds.
    if let Ok(path_str) = env::var("RDUMP_TEST_CONFIG_DIR") {
        return Some(PathBuf::from(path_str).join("rdump/config.toml"));
    }

    // If the override is not set, fall back to the default platform-specific directory.
    dirs::config_dir().map(|p| p.join("rdump/config.toml"))
}

/// Searches for a local `.rdump.toml` in the given directory and its parents.
fn find_local_config(start_dir: &Path) -> Option<PathBuf> {
    for ancestor in start_dir.ancestors() {
        let config_path = ancestor.join(".rdump.toml");
        if config_path.exists() {
            return Some(config_path);
        }
    }
    None
}

/// Finds and loads the configuration, merging global and local files.
pub fn load_config() -> Result<Config> {
    let mut final_config = Config::default();

    // 1. Load the global config file, if it exists.
    if let Some(global_config_path) = global_config_path() {
        if global_config_path.exists() {
            let global_config_str = fs::read_to_string(&global_config_path).with_context(|| {
                format!("Failed to read global config at {:?}", global_config_path)
            })?;
            let global_config: Config = toml::from_str(&global_config_str)?;
            final_config.presets.extend(global_config.presets);
        }
    }

    // 2. Find and load the local config file, if it exists.
    let current_dir = env::current_dir()?;
    if let Some(local_config_path) = find_local_config(&current_dir) {
        if local_config_path.exists() {
            let local_config_str = fs::read_to_string(&local_config_path).with_context(|| {
                format!("Failed to read local config at {:?}", local_config_path)
            })?;
            let local_config: Config = toml::from_str(&local_config_str)?;
            final_config.presets.extend(local_config.presets);
        }
    }

    Ok(final_config)
}

/// Saves the given config to the global configuration file.
pub fn save_config(config: &Config) -> Result<()> {
    let path = global_config_path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine global config path"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory at {:?}", parent))?;
    }

    let toml_string = toml::to_string_pretty(config)?;
    fs::write(&path, toml_string)
        .with_context(|| format!("Failed to write global config to {:?}", path))?;

    println!("Successfully saved config to {:?}", path);
    Ok(())
}

// The unit tests for config.rs remain the same and will still pass.
#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    lazy_static! {
        static ref ENV_MUTEX: Mutex<()> = Mutex::new(());
    }

    #[test]
    fn test_find_local_config_in_parent() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let root = tempdir().unwrap();
        let sub = root.path().join("sub");
        fs::create_dir(&sub).unwrap();

        let config_path = root.path().join(".rdump.toml");
        fs::File::create(&config_path).unwrap();

        let found_path = find_local_config(&sub).unwrap();
        assert_eq!(found_path, config_path);
    }

    #[test]
    fn test_find_local_config_not_found() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let root = tempdir().unwrap();
        assert!(find_local_config(root.path()).is_none());
    }

    #[test]
    fn test_load_config_merging_and_overriding() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let test_dir = tempdir().unwrap();

        let fake_home_dir = test_dir.path().join("home");
        let global_config_dir = fake_home_dir.join("rdump");
        fs::create_dir_all(&global_config_dir).unwrap();
        let global_config_path = global_config_dir.join("config.toml");
        let mut global_file = fs::File::create(&global_config_path).unwrap();
        writeln!(
            global_file,
            r#"
            [presets]
            rust = "ext:rs"
            docs = "ext:md"
        "#
        )
        .unwrap();

        let project_dir = test_dir.path().join("project");
        fs::create_dir(&project_dir).unwrap();
        let local_config_path = project_dir.join(".rdump.toml");
        let mut local_file = fs::File::create(&local_config_path).unwrap();
        writeln!(
            local_file,
            r#"
            [presets]
            docs = "ext:md | ext:txt"
            scripts = "ext:sh"
        "#
        )
        .unwrap();

        env::set_var("RDUMP_TEST_CONFIG_DIR", fake_home_dir.to_str().unwrap());
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&project_dir).unwrap();
        let config = load_config().unwrap();
        env::set_current_dir(&original_dir).unwrap();

        assert_eq!(config.presets.len(), 3);
        assert_eq!(config.presets.get("rust").unwrap(), "ext:rs");
        assert_eq!(config.presets.get("scripts").unwrap(), "ext:sh");
        assert_eq!(config.presets.get("docs").unwrap(), "ext:md | ext:txt");

        env::remove_var("RDUMP_TEST_CONFIG_DIR");
    }
}
