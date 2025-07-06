use crate::{config, ColorChoice, SearchArgs};
use anyhow::anyhow;
use anyhow::Result;
use atty::Stream;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tree_sitter::Range;

use crate::evaluator::{Evaluator, FileContext, MatchResult};
use crate::formatter;
use crate::parser;
use crate::predicates;

/// The main entry point for the `search` command.
pub fn run_search(mut args: SearchArgs) -> Result<()> {
    // --- Handle Shorthand Flags ---
    if args.no_headers {
        args.format = crate::Format::Cat;
    }
    if args.find {
        args.format = crate::Format::Find;
    }

    // --- Load Config and Build Query ---
    let config = config::load_config()?;
    let mut final_query = args.query.take().unwrap_or_default();

    for preset_name in args.preset.iter().rev() {
        let preset_query = config
            .presets
            .get(preset_name)
            .ok_or_else(|| anyhow!("Preset '{}' not found", preset_name))?;

        if final_query.is_empty() {
            final_query = format!("({})", preset_query);
        } else {
            final_query = format!("({}) & {}", preset_query, final_query);
        }
    }

    if final_query.is_empty() {
        return Err(anyhow!(
            "Empty query. Provide a query string or use a preset."
        ));
    }

    // --- 1. Find initial candidates ---
    let candidate_files =
        get_candidate_files(&args.root, args.no_ignore, args.hidden, args.max_depth)?;

    // --- 2. Parse query ---
    let ast = parser::parse_query(&final_query)?;

    // --- 3. Pre-filtering Pass (Metadata) ---
    // This pass uses a special evaluator with only fast metadata predicates.
    // It quickly reduces the number of files needing full evaluation.
    let metadata_registry = predicates::create_metadata_predicate_registry();
    let pre_filter_evaluator = Evaluator::new(ast.clone(), metadata_registry);

    let pre_filtered_files: Vec<PathBuf> = candidate_files
        .into_iter() // This pass is not parallel, it's fast enough.
        .filter(|path| {
            let mut context = FileContext::new(path.clone());
            match pre_filter_evaluator.pre_filter_evaluate(&mut context) {
                Ok(is_match) => is_match,
                Err(e) => {
                    eprintln!("Error during pre-filter on {}: {}", path.display(), e);
                    false
                }
            }
        })
        .collect();

    // --- Determine if color should be used ---
    let use_color = match args.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => atty::is(Stream::Stdout),
    };

    // --- 4. Main Evaluation Pass (Content + Semantic) ---
    // This pass uses the full evaluator on the smaller, pre-filtered set of files.
    let full_registry = predicates::create_predicate_registry();
    let evaluator = Evaluator::new(ast, full_registry);

    let mut matching_files: Vec<(PathBuf, Vec<Range>)> = pre_filtered_files
        .par_iter()
        .filter_map(|path| {
            let mut context = FileContext::new(path.clone());
            match evaluator.evaluate(&mut context) {
                Ok(MatchResult::Boolean(true)) => Some((path.clone(), Vec::new())),
                Ok(MatchResult::Boolean(false)) => None,
                Ok(MatchResult::Hunks(hunks)) => {
                    if hunks.is_empty() {
                        None
                    } else {
                        Some((path.clone(), hunks))
                    }
                }
                Err(e) => {
                    eprintln!("Error evaluating file {}: {}", path.display(), e);
                    None
                }
            }
        })
        .collect();

    matching_files.sort_by(|a, b| a.0.cmp(&b.0));

    // --- 5. Format and print results ---
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
        use_color,
        args.context.unwrap_or(0),
    )?;

    Ok(())
}

/// Walks the directory, respecting .gitignore, and applies our own smart defaults.
fn get_candidate_files(
    root: &PathBuf,
    no_ignore: bool,
    hidden: bool,
    max_depth: Option<usize>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut walker_builder = WalkBuilder::new(root);

    walker_builder.hidden(!hidden).max_depth(max_depth);

    if !no_ignore {
        // Layer 1: Our "sane defaults". These have the lowest precedence.
        let default_ignores = "
           # Default rdump ignores
           node_modules/
           target/
           dist/
           build/
           .git/
           .svn/
           .hg/
           *.pyc
           __pycache__/
       ";
        let mut temp_ignore = NamedTempFile::new()?;
        write!(temp_ignore, "{}", default_ignores)?;
        walker_builder.add_ignore(temp_ignore.path());

        // Layer 2: A user's custom global ignore file.
        if let Some(global_ignore_path) = dirs::config_dir().map(|p| p.join("rdump/ignore")) {
            if global_ignore_path.exists() {
                if let Some(err) = walker_builder.add_ignore(global_ignore_path) {
                    eprintln!("Warning: could not add global ignore file: {}", err);
                }
            }
        }

        // Layer 3: A user's custom project-local .rdumpignore file.
        walker_builder.add_custom_ignore_filename(".rdumpignore");

        // Layer 4: Standard .gitignore files.
        walker_builder.git_global(true);
        walker_builder.git_ignore(true);
    } else {
        // If --no-ignore is passed, disable everything.
        walker_builder.ignore(false);
    }

    for result in walker_builder.build() {
        let entry = result?;
        if entry.file_type().map_or(false, |ft| ft.is_file()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn get_sorted_file_names(
        root: &PathBuf,
        no_ignore: bool,
        hidden: bool,
        max_depth: Option<usize>,
    ) -> Vec<String> {
        let mut paths = get_candidate_files(root, no_ignore, hidden, max_depth).unwrap();
        paths.sort();
        paths
            .into_iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    #[test]
    fn test_custom_rdumpignore_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let mut ignore_file = fs::File::create(root.join(".rdumpignore")).unwrap();
        writeln!(ignore_file, "*.log").unwrap();
        fs::File::create(root.join("app.js")).unwrap();
        fs::File::create(root.join("app.log")).unwrap();

        let files = get_sorted_file_names(&root.to_path_buf(), false, false, None);
        assert_eq!(files, vec!["app.js"]);
    }

    #[test]
    fn test_unignore_via_rdumpignore() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let node_modules = root.join("node_modules");
        fs::create_dir(&node_modules).unwrap();
        fs::File::create(node_modules.join("some_dep.js")).unwrap();
        fs::File::create(root.join("app.js")).unwrap();

        let mut ignore_file = fs::File::create(root.join(".rdumpignore")).unwrap();
        writeln!(ignore_file, "!node_modules/").unwrap();

        let files = get_sorted_file_names(&root.to_path_buf(), false, false, None);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"app.js".to_string()));
        assert!(files.contains(
            &"node_modules/some_dep.js"
                .to_string()
                .replace('/', &std::path::MAIN_SEPARATOR.to_string())
        ));
    }
}
