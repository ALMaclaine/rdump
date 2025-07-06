mod evaluator;
mod formatter;
mod parser;

use anyhow::Result;
use clap::Parser;
use evaluator::Evaluator;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::PathBuf;

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

    /// Do not print file path headers in the output.
    #[arg(long)]
    no_headers: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // --- 1. Find all potential files to check ---
    let candidate_files = get_candidate_files(&cli.root)?;

    // --- 2. Parse the query string ---
    let ast = parser::parse_query(&cli.query)?;

    // --- 3. Evaluate files against the query in parallel ---
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

    // Sort the final list for deterministic output
    matching_files.sort();

    // --- 4. Format and print results ---
    // Get a handle to stdout and pass it to the testable formatter.
    let mut stdout = std::io::stdout();
    formatter::print_output(
        &mut stdout,
        &matching_files,
        cli.line_numbers,
        cli.no_headers,
    )?;

    Ok(())
}

/// Walks the directory, respects .gitignore, and applies our own smart defaults to return a list of files.
fn get_candidate_files(root: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let mut override_builder = OverrideBuilder::new(root);
    override_builder.add("!node_modules/")?;
    override_builder.add("!target/")?;
    override_builder.add("!.git/")?;
    let overrides = override_builder.build()?;

    let walker = WalkBuilder::new(root).overrides(overrides).build();

    for result in walker {
        let entry = result?;
        if entry.file_type().map_or(false, |ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}
