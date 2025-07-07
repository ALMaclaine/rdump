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
use commands::{lang::run_lang, preset::run_preset, search::run_search};

// These structs and enums define the public API of our CLI.
// They need to be public so the `commands` modules can use them.
#[derive(Parser, Debug)]
#[command(
    version,
    about = "A fast, expressive, code-aware tool to find and dump file contents."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for files using a query (default command).
    #[command(visible_alias = "s")]
    Search(SearchArgs),
    /// List supported languages and their available predicates.
    #[command(visible_alias = "l")]
    Lang(LangArgs),
    /// Manage saved presets.
    #[command(visible_alias = "p")]
    Preset(PresetArgs),
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Parser, Debug)]
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
    #[doc = "CODE-AWARE PREDICATES for supported languages:"]
    ///   def:<str>          - A generic definition (class, struct, enum, etc.)
    ///   func:<str>         - A function or method
    ///   import:<str>       - An import or use statement
    ///   call:<str>         - A function or method call site
    ///
    /// GRANULAR DEFINITIONS:
    ///   class:<str>        - A class definition
    ///   struct:<str>       - A struct definition
    ///   enum:<str>         - An enum definition
    ///   interface:<str>    - An interface definition
    ///   trait:<str>        - A trait definition
    ///   type:<str>         - A type alias
    ///
    /// SYNTACTIC CONTENT:
    ///   comment:<str>      - Text inside a comment (e.g., "TODO", "FIXME")
    ///   str:<str>          - Text inside a string literal
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
    #[arg(long, value_enum, default_value_t = Format::Hunks)]
    pub format: Format,
    #[arg(long)]
    pub no_ignore: bool,
    #[arg(long)]
    pub hidden: bool,
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto, help = "When to use syntax highlighting")]
    pub color: ColorChoice,
    #[arg(long)]
    pub max_depth: Option<usize>,
    #[arg(
        long,
        short = 'C',
        value_name = "LINES",
        help = "Show LINES of context around matches for --format=hunks"
    )]
    pub context: Option<usize>,

    /// List files with metadata instead of dumping content.
    #[arg(long)]
    pub find: bool,
}

#[derive(Parser, Debug)]
pub struct LangArgs {
    #[command(subcommand)]
    pub action: Option<LangAction>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum LangAction {
    /// List all supported languages.
    List,
    /// Describe the predicates available for a specific language.
    Describe { language: String },
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
    /// Show only the specific code blocks ("hunks") that match a semantic query
    Hunks,
    /// Human-readable markdown with file headers
    Markdown,
    /// Machine-readable JSON
    Json,
    /// A simple list of matching file paths
    Paths,
    /// Raw concatenated file content, for piping
    Cat,
    /// `ls`-like output with file metadata
    Find,
}

/// The main entry point.
/// Its only job is to parse the CLI and delegate to the correct command module.
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search(args) => run_search(args),
        Commands::Lang(args) => {
            // Default to `list` if no subcommand is given for `lang`
            let action = args.action.unwrap_or(LangAction::List);
            run_lang(action)
        }
        Commands::Preset(args) => run_preset(args.action),
    }
}
