// Declare all our modules
mod commands;
mod config;
mod evaluator;
mod formatter;
mod parser;
mod predicates;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

// Bring our command functions into scope
use commands::{preset::run_preset, search::run_search};

// These structs and enums define the public API of our CLI.
// They need to be public so the `commands` modules can use them.
#[derive(Parser, Debug)]
#[command(version, about = "A fast, expressive, code-aware tool to find and dump file contents.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for files using a query (default command).
    #[command(visible_alias = "s")]
    Search(SearchArgs),
    /// Manage saved presets.
    #[command(visible_alias = "p")]
    Preset(PresetArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct SearchArgs {
    /// The query string to search for, using rdump Query Language (RQL).
    ///
    /// RQL supports logical operators (&, |, !), parentheses, and key:value predicates.
    /// Values with spaces must be quoted (e.g., contains:'fn main').
    ///
    /// METADATA PREDICATES:
    ///   ext:<str>          - File extension (e.g., "rs", "toml")
    ///   name:<glob>        - File name glob pattern (e.g., "test_*.rs")
    ///   path:<str>         - Substring in the full file path
    ///   size:[>|<]<num>[kb|mb] - File size (e.g., ">10kb")
    ///   modified:[>|<]<num>[h|d|w] - Modified time (e.g., "<2d")
    ///
    /// CONTENT PREDICATES:
    ///   contains:<str>     - Literal string a file contains
    ///   matches:<regex>    - Regular expression a file's content matches
    ///
    /// CODE-AWARE PREDICATES (for Rust, Python):
    ///   def:<str>          - A definition (class, struct, enum, trait)
    ///   func:<str>         - A function or method definition
    ///   import:<str>       - An import or use statement
    #[arg(verbatim_doc_comment)]
    pub query: Option<String>,
    #[arg(long, short)]
    pub preset: Vec<String>,
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(short, long)]
    pub line_numbers: bool,
    #[arg(long)]
    pub no_headers: bool,
    #[arg(long, value_enum, default_value_t = Format::Markdown)]
    pub format: Format,
    #[arg(long)]
    pub no_ignore: bool,
    #[arg(long)]
    pub hidden: bool,
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// List files with metadata instead of dumping content.
    #[arg(long)]
    pub find: bool,
}

#[derive(Parser, Debug)]
pub struct PresetArgs {
    #[command(subcommand)]
    pub action: PresetAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum PresetAction {
    /// List all available presets.
    List,
    /// Add or update a preset in the global config file.
    Add {
        #[arg(required = true)]
        name: String,
        #[arg(required = true)]
        query: String,
    },
    /// Remove a preset from the global config file.
    Remove {
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Format {
    Markdown,
    Json,
    Paths,
    Cat,
    Find,
}

/// The main entry point.
/// Its only job is to parse the CLI and delegate to the correct command module.
fn main() -> Result<()> {
    let mut cli = Cli::parse();

    match &mut cli.command {
        Commands::Search(args) => {
            // --- Handle Shorthand Flags ---
            if args.no_headers {
                args.format = Format::Cat;
            }
            if args.find {
                args.format = Format::Find;
            }
            run_search(args.clone())
        }
        Commands::Preset(args) => run_preset(args.action.clone()),
    }
}