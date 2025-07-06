use anyhow::Result;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::evaluator::Evaluator;
use crate::formatter;
use crate::parser;
use crate::{config, SearchArgs};

/// The main entry point for the `search` command.
pub fn run_search(mut args: SearchArgs) -> Result<()> {
    // --- Load Config and Build Query ---
    let config = config::load_config()?;
    let mut final_query = args.query.take().unwrap_or_default();

    for preset_name in args.preset.iter().rev() {
        let preset_query = config.presets.get(preset_name)
            .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_name))?;

        if final_query.is_empty() {
            final_query = format!("({})", preset_query);
        } else {
            final_query = format!("({}) & {}", preset_query, final_query);
        }
    }

    if final_query.is_empty() {
        return Err(anyhow::anyhow!("Empty query. Provide a query string or use a preset."));
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

    Ok(())
}

/// Walks the directory, respecting .gitignore, and applies our own smart defaults.
// This is now a private helper function within the search module.
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
