use crate::config::{self, Config};
use crate::PresetAction;
use anyhow::{anyhow, Result};
use std::fs; // We'll need to make PresetAction public

/// The main entry point for the `preset` command.
pub fn run_preset(action: PresetAction) -> Result<()> {
    match action {
        PresetAction::List => {
            let config = config::load_config()?;
            if config.presets.is_empty() {
                println!("No presets found.");
            } else {
                println!("Available presets:");
                let max_len = config.presets.keys().map(|k| k.len()).max().unwrap_or(0);
                for (name, query) in config.presets {
                    println!("  {name:<max_len$} : {query}");
                }
            }
        }
        PresetAction::Add { name, query } => {
            let path = config::global_config_path()
                .ok_or_else(|| anyhow!("Could not determine global config path"))?;

            let mut config = if path.exists() {
                let config_str = fs::read_to_string(&path)?;
                toml::from_str(&config_str)?
            } else {
                Config::default()
            };

            println!("Adding/updating preset '{name}'...");
            config.presets.insert(name, query);
            config::save_config(&config)?;
        }
        PresetAction::Remove { name } => {
            let path = config::global_config_path()
                .ok_or_else(|| anyhow!("Could not determine global config path"))?;

            if !path.exists() {
                return Err(anyhow!(
                    "Global config file does not exist. No presets to remove."
                ));
            }

            let mut config: Config = toml::from_str(&fs::read_to_string(&path)?)?;

            if config.presets.remove(&name).is_some() {
                println!("Removing preset '{name}'...");
                config::save_config(&config)?;
            } else {
                return Err(anyhow!("Preset '{}' not found in global config.", name));
            }
        }
    }
    Ok(())
}
