use crate::{config, ColorChoice, SearchArgs};
use anyhow::anyhow;
use anyhow::Result;
use atty::Stream;
use ignore::{WalkBuilder, DirEntry};
use rayon::prelude::*;
use std::fs::File;
 use std::io::{self, Write};
 use std::path::PathBuf;
use tempfile::NamedTempFile;

 use crate::evaluator::{Evaluator, FileContext};
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
    let mut matching_files: Vec<PathBuf> = candidate_files
        .par_iter()
       .filter(|path| {
           // This closure now only returns true or false, reducing allocations.
           let mut context = FileContext::new((*path).clone());
           match evaluator.evaluate(&mut context) {
               Ok(true) => true,
               Ok(false) => false,
               Err(e) => {
                   eprintln!("Error evaluating file {}: {}", path.display(), e);
                   false
               }
           }
       })
       .map(|path| path.clone()) // Clones only the paths that passed the filter.
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

    walker_builder
        .ignore(!no_ignore)
        .git_global(!no_ignore)
        .git_ignore(!no_ignore)
        .hidden(!hidden)
        .max_depth(max_depth);

    // Add our own sane defaults for ignored directories, unless --no-ignore is passed.
    if !no_ignore {
       // Create an in-memory temporary file with our default ignore patterns.
       let mut temp_ignore = NamedTempFile::new()?;
       writeln!(temp_ignore, "node_modules/")?;
       writeln!(temp_ignore, "target/")?;
       writeln!(temp_ignore, "dist/")?;
       writeln!(temp_ignore, "build/")?;
       writeln!(temp_ignore, ".git/")?;
       writeln!(temp_ignore, ".svn/")?;
       writeln!(temp_ignore, ".hg/")?;
       writeln!(temp_ignore, "*.pyc")?;
       writeln!(temp_ignore, "__pycache__/")?;

       // Add this temp file to the WalkBuilder's ignore list.
       walker_builder.add_ignore(temp_ignore.path());
    }

    // This closure will be used to filter entries.
    let is_file = |entry: &DirEntry| -> bool {
        entry.file_type().map_or(false, |ft| ft.is_file())
    };

    for result in walker_builder.build().filter_map(Result::ok) {
        if is_file(&result) {
            files.push(result.into_path());
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

    fn test_get_candidates_with_max_depth() {
        let (_dir, root) = create_test_fs();
        // Depth 1 is the root directory itself.
        // Depth 2 is the root + immediate children.
        let files = get_sorted_file_names(&root, false, false, Some(2));
        // Should find file_a.txt and file_b.txt which is at depth 2 (root -> sub -> file_b)
        assert_eq!(files, vec!["file_a.txt", "sub/file_b.txt"]);
    }

   #[test]
   fn test_get_candidates_ignores_node_modules_by_default() {
       // Setup a directory with node_modules but NO .gitignore
       let dir = tempdir().unwrap();
       let root = dir.path().to_path_buf();
       fs::File::create(root.join("app.js")).unwrap();
       fs::create_dir_all(root.join("node_modules/express")).unwrap();
       fs::File::create(root.join("node_modules/express/index.js")).unwrap();

       // Default behavior: should ignore node_modules
       let files_default = get_sorted_file_names(&root, false, false, None);
       assert_eq!(files_default, vec!["app.js"]);

       // With --no-ignore: should find files inside node_modules
       let files_no_ignore = get_sorted_file_names(&root, true, false, None);
       let expected: HashSet<String> = [
           "app.js".to_string(),
           "node_modules/express/index.js".to_string(),
       ]
       .iter()
       .cloned()
       .collect();
       let found: HashSet<String> = files_no_ignore.into_iter().collect();
       assert_eq!(found, expected);
   }
}