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

/// The main entry point for the `search` command.
pub fn run_search(mut args: SearchArgs) -> Result<()> {
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

    // --- 1. Find candidates ---
    let candidate_files =
        get_candidate_files(&args.root, args.no_ignore, args.hidden, args.max_depth)?;

    // --- 2. Parse query ---
    let ast = parser::parse_query(&final_query)?;

   // --- Determine if color should be used ---
   let use_color = match args.color {
       ColorChoice::Always => true,
       ColorChoice::Never => false,
       ColorChoice::Auto => atty::is(Stream::Stdout),
   };

    // --- 3. Evaluate files ---
    let evaluator = Evaluator::new(ast);
   let mut matching_files: Vec<(PathBuf, Vec<Range>)> = candidate_files
        .par_iter()
        .filter_map(|path| {
            let mut context = FileContext::new(path.clone());
            match evaluator.evaluate(&mut context) {
               Ok(MatchResult::Boolean(true)) => {
                   // For boolean matches, we don't have specific hunks, so we pass an empty Vec.
                   // The formatter will treat this as "the whole file".
                   Some((path.clone(), Vec::new()))
               }
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
       use_color,
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
       // A user can override these with `!` in their own ignore files.
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
       // This has high precedence.
        walker_builder.add_custom_ignore_filename(".rdumpignore");

       // Layer 4: Standard .gitignore files, which have the highest project-specific precedence.
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

// Add to the bottom of rdump/src/commands/search.rs

#[cfg(test)]
mod tests {
// ... (existing tests are unchanged)
// ...
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_fs() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // Create files and directories
        fs::File::create(root.join("file_a.txt")).unwrap();
        fs::File::create(root.join(".hidden_file")).unwrap();

        fs::create_dir(root.join("sub")).unwrap();
        fs::File::create(root.join("sub/file_b.txt")).unwrap();

        fs::create_dir_all(root.join("sub/sub2")).unwrap();
        fs::File::create(root.join("sub/sub2/file_c.log")).unwrap();

        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::File::create(root.join("target/debug/app.exe")).unwrap();

        fs::create_dir(root.join("logs")).unwrap();
        fs::File::create(root.join("logs/yesterday.log")).unwrap();

        let mut gitignore = fs::File::create(root.join(".gitignore")).unwrap();
        writeln!(gitignore, "*.log").unwrap();
        writeln!(gitignore, "logs/").unwrap();
        writeln!(gitignore, "target/").unwrap();

        (dir, root)
    }

    // Helper to run get_candidate_files and return a sorted list of file names
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

// ... (existing test functions)

    // ... (existing tests are unchanged)
// ...
// ... existing tests ...
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
       // This test verifies that a user can override our "sane defaults".
       let dir = tempdir().unwrap();
       let root = dir.path();

       // Create a node_modules dir, which is ignored by default.
       let node_modules = root.join("node_modules");
       fs::create_dir(&node_modules).unwrap();
       fs::File::create(node_modules.join("some_dep.js")).unwrap();
       fs::File::create(root.join("app.js")).unwrap();

       // Create an ignore file that explicitly re-includes node_modules.
       let mut ignore_file = fs::File::create(root.join(".rdumpignore")).unwrap();
       writeln!(ignore_file, "!node_modules/").unwrap();

       // Run the search. Both files should now be found.
       let files = get_sorted_file_names(&root.to_path_buf(), false, false, None);
       assert_eq!(files.len(), 2);
       assert!(files.contains(&"app.js".to_string()));
       assert!(files.contains(&"node_modules/some_dep.js".to_string().replace('/', &std::path::MAIN_SEPARATOR.to_string())));
   }
}