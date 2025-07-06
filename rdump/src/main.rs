mod evaluator;
mod formatter;
mod parser;

use std::fs::File;
use std::io::{self, Write};

use anyhow::Result;
use clap::{Parser, ValueEnum};
use evaluator::Evaluator;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::PathBuf;



// NEW: An enum to represent our output formats for clap
#[derive(Debug, Clone, ValueEnum)]
enum Format {
    Markdown,
    Json,
    Paths,
    Cat,
}

/// A fast, expressive tool to find and dump file contents for LLM context using a query language.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The query string to search for files.
    #[arg(required = true)]
    query: String,

    /// The root directory to start the search from.
    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    /// Output file path. If not provided, output is written to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Add line numbers to the output.
    #[arg(short, long)]
    line_numbers: bool,

    /// A shorthand for '--format cat'.
    #[arg(long)]
    no_headers: bool,

    /// The output format for the results.
    #[arg(long, value_enum, default_value_t = Format::Markdown)]
    format: Format,

    // --- NEW FLAGS ---
    /// Do not respect .gitignore and other ignore files.
    #[arg(long)]
    no_ignore: bool,

    /// Search hidden files and directories.
    #[arg(long)]
    hidden: bool,

    /// Set the maximum search depth.
    #[arg(long)]
    max_depth: Option<usize>,
}



fn main() -> Result<()> {
    let mut cli = Cli::parse();

    // --- Handle `--no-headers` shorthand ---
    if cli.no_headers {
        cli.format = Format::Cat;
    }

    // --- 1. Find candidates ---
    // MODIFIED: Pass the new flags to the function.
    let candidate_files = get_candidate_files(
        &cli.root,
        cli.no_ignore,
        cli.hidden,
        cli.max_depth,
    )?;

    // --- 2. Parse query ---
    let ast = parser::parse_query(&cli.query)?;

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
    // This `Box<dyn Write>` lets us decide at runtime whether to write
    // to stdout or a file, without changing the formatter's code.
    let mut writer: Box<dyn Write> = if let Some(output_path) = &cli.output {
        Box::new(File::create(output_path)?)
    } else {
        Box::new(io::stdout())
    };

    formatter::print_output(
        &mut writer,
        &matching_files,
        &cli.format,
        cli.line_numbers,
    )?;

    Ok(())
}

/// Walks the directory, respecting .gitignore, and applies our own smart defaults to return a list of files.
// MODIFIED: The function now takes the flags it needs from the Cli struct.
fn get_candidate_files(
    root: &PathBuf,
    no_ignore: bool,
    hidden: bool,
    max_depth: Option<usize>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let mut override_builder = OverrideBuilder::new(root);
    // Only apply our smart defaults if ignore files are being used.
    if !no_ignore {
        override_builder.add("!node_modules/")?;
        override_builder.add("!target/")?;
        override_builder.add("!.git/")?;
    }
    let overrides = override_builder.build()?;

    // --- MODIFIED: Configure WalkBuilder with our new flags ---
    let mut walker_builder = WalkBuilder::new(root);
    walker_builder
        .overrides(overrides)
        .ignore(!no_ignore) // The .ignore(bool) method defaults to true. We pass the opposite of our flag.
        .hidden(!hidden)    // The .hidden(bool) method defaults to true (hides files). We pass the opposite.
        .max_depth(max_depth);

    for result in walker_builder.build() {
        let entry = result?;
        if entry.file_type().map_or(false, |ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}
