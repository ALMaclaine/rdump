use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// This struct represents the structure of our TOML config file.
// `#[derive(Deserialize)]` tells serde how to create this struct from TOML text.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    // `#[serde(default)]` ensures that if the `presets` table is missing,
    // we just get an empty HashMap instead of an error.
    #[serde(default)]
    pub presets: HashMap<String, String>,
}

/// Finds and loads the configuration, merging global and local files.
pub fn load_config() -> Result<Config> {
    let mut final_config = Config::default();

    // 1. Load the global config file, if it exists.
    if let Some(global_config_path) = global_config_path() {
        if global_config_path.exists() {
            let global_config_str = fs::read_to_string(&global_config_path)
                .with_context(|| format!("Failed to read global config at {:?}", global_config_path))?;
            let global_config: Config = toml::from_str(&global_config_str)?;
            final_config.presets.extend(global_config.presets);
        }
    }

    // 2. Find and load the local config file, if it exists.
    // Local presets will overwrite global ones with the same name.
    if let Some(local_config_path) = find_local_config() {
        if local_config_path.exists() {
            let local_config_str = fs::read_to_string(&local_config_path)
                .with_context(|| format!("Failed to read local config at {:?}", local_config_path))?;
            let local_config: Config = toml::from_str(&local_config_str)?;
            final_config.presets.extend(local_config.presets);
        }
    }

    Ok(final_config)
}

/// Returns the path to the global configuration file.
pub fn global_config_path() -> Option<PathBuf> {
    // Use the `dirs` crate to find the conventional config directory.
    dirs::config_dir().map(|p| p.join("rdump/config.toml"))
}

/// Searches for a local `.rdump.toml` in the current directory and its parents.
fn find_local_config() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    for ancestor in current_dir.ancestors() {
        let config_path = ancestor.join(".rdump.toml");
        if config_path.exists() {
            return Some(config_path);
        }
    }
    None
}

/// Saves the given config to the global configuration file.
pub fn save_config(config: &Config) -> Result<()> {
    let path = global_config_path().ok_or_else(|| anyhow::anyhow!("Could not determine global config path"))?;

    // Ensure the parent directory exists.
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
