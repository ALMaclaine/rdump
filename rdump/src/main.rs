mod config; // <-- Add our new module
mod evaluator;
mod formatter;
mod parser;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use evaluator::Evaluator;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

// This is the top-level command structure.
#[derive(Parser, Debug)]
#[command(version, about = "A fast, expressive tool to find and dump file contents.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// All the different things our program can do.
// For now, it's just `Search` (the default) and `Preset`.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Search for files using a query (default command).
    #[command(visible_alias = "s")]
    Search(SearchArgs),
    /// Manage saved presets.
    #[command(visible_alias = "p")]
    Preset(PresetArgs),
}

// Arguments for the `Search` command. These are our familiar flags.
#[derive(Parser, Debug)]
struct SearchArgs {
    /// The query string to search for files.
    #[arg()]
    query: Option<String>,

    /// Use a saved preset as the base query.
    #[arg(long, short)]
    preset: Vec<String>, // Can be used multiple times

    #[arg(short, long, default_value = ".")]
    root: PathBuf,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(short, long)]
    line_numbers: bool,
    #[arg(long)]
    no_headers: bool,
    #[arg(long, value_enum, default_value_t = Format::Markdown)]
    format: Format,
    #[arg(long)]
    no_ignore: bool,
    #[arg(long)]
    hidden: bool,
    #[arg(long)]
    max_depth: Option<usize>,
}

// Arguments for the `Preset` command
#[derive(Parser, Debug)]
struct PresetArgs {
    #[command(subcommand)]
    action: PresetAction,
}

// MODIFIED: Added Add and Remove actions with their arguments
#[derive(Subcommand, Debug)]
enum PresetAction {
    /// List all available presets.
    List,
    /// Add or update a preset in the global config file.
    Add {
        /// The name of the preset (e.g., rust_src).
        #[arg(required = true)]
        name: String,
        /// The query string for the preset.
        #[arg(required = true)]
        query: String,
    },
    /// Remove a preset from the global config file.
    Remove {
        /// The name of the preset to remove.
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum Format {
    Markdown,
    Json,
    Paths,
    Cat,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // If the command is `Search`, we run our main search logic.
        Commands::Search(mut args) => {
            // --- Load Config and Build Query ---
            let config = config::load_config()?;

            // Start with the query from the command line, if it exists.
            let mut final_query = args.query.clone().unwrap_or_default();

            // Prepend presets to the query.
            for preset_name in args.preset.iter().rev() {
                let preset_query = config.presets.get(preset_name)
                    .ok_or_else(|| anyhow!("Preset '{}' not found", preset_name))?;

                if final_query.is_empty() {
                    final_query = format!("({})", preset_query);
                } else {
                    final_query = format!("({}) & {}", preset_query, final_query);
                }
            }

            if final_query.is_empty() {
                return Err(anyhow!("Empty query. Provide a query string or use a preset."));
            }

            // --- Handle `--no-headers` shorthand ---
            if args.no_headers {
                args.format = Format::Cat;
            }

            // --- 1. Find candidates ---
            let candidate_files = get_candidate_files(
                &args.root,
                args.no_ignore,
                args.hidden,
                args.max_depth,
            )?;

            // --- 2. Parse query ---
            let ast = parser::parse_query(&final_query)?;

            // --- 3. Evaluate files ---
            let evaluator = Evaluator::new(&ast);
            let mut matching_files: Vec<PathBuf> = candidate_files
                .par_iter()
                .filter_map(|path| match evaluator.evaluate(path) {
                    Ok(true) => Some(path.clone()),
                    Ok(false) => None,
                    Err(e) => {
                        eprintln!("Error evaluating file {}: {}", path.display(), e);
                        None
                    }
                })
                .collect();

            matching_files.sort();

            // --- 4. Format and print results ---
            let mut writer: Box<dyn Write> = if let Some(output_path) = &args.output {
                Box::new(File::create(output_path)?)
            } else {
                Box::new(io::stdout())
            };

            formatter::print_output(
                &mut writer,
                &matching_files,
                &args.format,
                args.line_numbers,
            )?;
        }
        // If the command is `Preset`, we handle its actions.
        Commands::Preset(args) => match args.action {
            PresetAction::List => {
                let config = config::load_config()?;
                if config.presets.is_empty() {
                    println!("No presets found.");
                } else {
                    println!("Available presets:");
                    // Find the longest key for alignment
                    let max_len = config.presets.keys().map(|k| k.len()).max().unwrap_or(0);
                    for (name, query) in config.presets {
                        println!("  {:<width$} : {}", name, query, width = max_len);
                    }
                }
            }
            PresetAction::Add { name, query } => {
                // We only ever add to the global config, not a local one.
                let path = config::global_config_path()
                    .ok_or_else(|| anyhow!("Could not determine global config path"))?;

                // Read the existing global config, or create a new one.
                let mut config = if path.exists() {
                    let config_str = fs::read_to_string(&path)?;
                    toml::from_str(&config_str)?
                } else {
                    config::Config::default()
                };

                println!("Adding/updating preset '{}'...", name);
                config.presets.insert(name, query);
                config::save_config(&config)?;
            }
            PresetAction::Remove { name } => {
                let path = config::global_config_path()
                    .ok_or_else(|| anyhow!("Could not determine global config path"))?;

                if !path.exists() {
                    return Err(anyhow!("Global config file does not exist. No presets to remove."));
                }

                let mut config: config::Config = toml::from_str(&fs::read_to_string(&path)?)?;

                if config.presets.remove(&name).is_some() {
                    println!("Removing preset '{}'...", name);
                    config::save_config(&config)?;
                } else {
                    return Err(anyhow!("Preset '{}' not found in global config.", name));
                }
            }
        },
    }

    Ok(())
}

// ... (get_candidate_files function remains the same) ...
fn get_candidate_files(
    root: &PathBuf,
    no_ignore: bool,
    hidden: bool,
    max_depth: Option<usize>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut override_builder = OverrideBuilder::new(root);
    if !no_ignore {
        override_builder.add("!node_modules/")?;
        override_builder.add("!target/")?;
        override_builder.add("!.git/")?;
    }
    let overrides = override_builder.build()?;
    let mut walker_builder = WalkBuilder::new(root);
    walker_builder
        .overrides(overrides)
        .ignore(!no_ignore)
        .hidden(!hidden)
        .max_depth(max_depth);
    for result in walker_builder.build() {
        let entry = result?;
        if entry.file_type().map_or(false, |ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}