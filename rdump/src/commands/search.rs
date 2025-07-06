use anyhow::Result;
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
    let mut walker_builder = WalkBuilder::new(root);

    walker_builder
        .ignore(!no_ignore)
        .git_ignore(!no_ignore)
        .hidden(!hidden)
        .max_depth(max_depth);

    if !no_ignore {
        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            walker_builder.add_ignore(gitignore_path);
        }
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
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().replace("\\", "/"))
            .collect()
    }

    #[test]
    fn test_get_candidates_default_behavior() {
        let (_dir, root) = create_test_fs();
        let files = get_sorted_file_names(&root, false, false, None);

        // Should find file_a.txt and file_b.txt
        // Should NOT find:
        // - .hidden_file (hidden)
        // - .gitignore (hidden)
        // - files in target/ (default override)
        // - *.log files (.gitignore)
        assert_eq!(files, vec!["file_a.txt", "sub/file_b.txt"]);
    }

    #[test]
    fn test_get_candidates_with_hidden() {
        let (_dir, root) = create_test_fs();
        let files = get_sorted_file_names(&root, false, true, None);

        // Should find .gitignore, .hidden_file, file_a.txt, file_b.txt
        let expected: HashSet<String> = [
            ".gitignore".to_string(),
            ".hidden_file".to_string(),
            "file_a.txt".to_string(),
            "sub/file_b.txt".to_string(),
        ]
        .iter()
        .cloned()
        .collect();
        let found: HashSet<String> = files.into_iter().collect();

        assert_eq!(found, expected);
    }

    #[test]
    fn test_get_candidates_with_no_ignore() {
        let (_dir, root) = create_test_fs();
        let files = get_sorted_file_names(&root, true, false, None);

        // Should find everything not hidden, including gitignored files
        // and files in the default-ignored 'target' dir.
        let expected: HashSet<String> = [
            "file_a.txt".to_string(),
            "sub/file_b.txt".to_string(),
            "sub/sub2/file_c.log".to_string(),
            "target/debug/app.exe".to_string(),
            "logs/yesterday.log".to_string(),
        ]
        .iter()
        .cloned()
        .collect();
        let found: HashSet<String> = files.into_iter().collect();

        assert_eq!(found, expected);
    }

    #[test]
    fn test_get_candidates_with_max_depth() {
        let (_dir, root) = create_test_fs();
        // Depth 1 is the root directory itself.
        // Depth 2 is the root + immediate children.
        let files = get_sorted_file_names(&root, false, false, Some(2));
        // Should find file_a.txt and file_b.txt which is at depth 2 (root -> sub -> file_b)
        assert_eq!(files, vec!["file_a.txt", "sub/file_b.txt"]);
    }
}