use anyhow::Result;
use rdump::{commands::search::run_search, ColorChoice, Format, SearchArgs};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Helper to create a default SearchArgs for testing.
/// We enable `no_ignore` and `hidden` to make tests self-contained and predictable.
fn create_test_args(root: &Path, query: &str) -> SearchArgs {
    SearchArgs {
        query: query.split(' ').map(String::from).collect(), // Split query for clap
        root: root.to_path_buf(),
        preset: vec![],
        output: None,
        line_numbers: false,
        no_headers: false,
        format: Format::Paths,
        no_ignore: true, // Crucial for hermetic tests
        hidden: true,    // Crucial for hermetic tests
        color: ColorChoice::Never,
        max_depth: None,
        context: None,
        find: false,
    }
}

/// Helper to run a search and return the relative paths of matching files.
/// NOTE: This now uses `run_search` and captures stdout, as `perform_search` is not public.
/// To make perform_search public, we'd need to adjust the `lib.rs` design.
/// For these tests, we will create a custom test helper that calls the full `run_search`
/// and returns the result, which is more of a true integration test.
///
/// Let's stick with the previous `perform_search` for simplicity and make it public.
/// The `lib.rs` change makes this possible. Let's re-import it.
use rdump::commands::search::perform_search;

fn run_test_search(root: &Path, query: &str) -> Result<Vec<String>> {
    let args = create_test_args(root, query);
    let results = perform_search(&args)?;
    let mut paths: Vec<String> = results
        .into_iter()
        .map(|(p, _)| {
            p.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/") // Normalize for Windows
        })
        .collect();
    paths.sort();
    Ok(paths)
}

/// Sets up a standard test project structure.
fn setup_test_project() -> Result<tempfile::TempDir> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("tests"))?;
    fs::create_dir_all(root.join("benches"))?;
    fs::create_dir_all(root.join("docs"))?;

    fs::write(
        root.join("src/user.rs"),
        "// TODO: Add more fields\nstruct User {}",
    )?;
    fs::write(root.join("src/order.rs"), "struct Order {}")?;
    fs::write(
        root.join("src/special.txt"),
        "the user's settings\nvalue * 2",
    )?;
    fs::write(root.join("tests/user_test.rs"), "fn test_user() {}")?;
    fs::write(root.join("benches/user.rs"), "fn bench_user() {}")?;
    fs::write(root.join("docs/api.md"), "# API Docs")?;

    Ok(dir)
}

#[test]
fn test_query_with_negated_group() -> Result<()> {
    let dir = setup_test_project()?;
    let root = dir.path();

    // Find all rust files that are NOT in the tests or benches directories.
    let query = "ext:rs & !(in:tests | in:benches)";
    let results = run_test_search(root, query)?;

    assert_eq!(results.len(), 2);
    assert_eq!(results, vec!["src/order.rs", "src/user.rs"]);
    Ok(())
}

#[test]
fn test_query_combining_semantic_and_content_predicates() -> Result<()> {
    let dir = setup_test_project()?;
    let root = dir.path();

    // Find a struct named 'User' that also has a 'TODO' comment.
    let query = "struct:User & comment:TODO";
    let results = run_test_search(root, query)?;
    assert_eq!(results, vec!["src/user.rs"]);

    // Find a struct named 'Order' that also has a 'TODO' comment (it doesn't).
    let query_no_match = "struct:Order & comment:TODO";
    let results_no_match = run_test_search(root, query_no_match)?;
    assert!(results_no_match.is_empty());

    Ok(())
}

#[test]
fn test_query_combining_metadata_and_semantic_predicates() -> Result<()> {
    let dir = setup_test_project()?;
    let root = dir.path();

    // Find a struct named 'User' but only within the 'src' directory.
    let query = "in:src & struct:User";
    let results = run_test_search(root, query)?;
    assert_eq!(results, vec!["src/user.rs"]);

    // Search for a function inside the 'docs' directory (it won't find one).
    let query_no_match = "in:docs & func:test_user";
    let results_no_match = run_test_search(root, query_no_match)?;
    assert!(results_no_match.is_empty());

    Ok(())
}

#[test]
#[cfg(unix)] // This test relies on Unix-style permissions and paths
fn test_search_fails_on_unwritable_output_path() -> Result<()> {
    let dir = setup_test_project()?;
    let root = dir.path();
    let unwritable_dir = root.join("unwritable");
    fs::create_dir(&unwritable_dir)?;

    // Make directory read-only
    let mut perms = fs::metadata(&unwritable_dir)?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(&unwritable_dir, perms)?;

    let output_path = unwritable_dir.join("output.txt");

    let mut args = create_test_args(root, "ext:rs");
    args.output = Some(output_path);

    // Call the full `run_search` which attempts to create the file
    let result = run_search(args);

    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("Permission denied") || error_message.contains("os error 13"),
        "Error message should indicate a permission issue"
    );

    // Set back to writable so tempdir can clean up
    let mut perms = fs::metadata(&unwritable_dir)?.permissions();
    perms.set_readonly(false);
    fs::set_permissions(&unwritable_dir, perms)?;

    Ok(())
}

#[test]
fn test_query_with_literal_glob_character() -> Result<()> {
    let dir = setup_test_project()?;
    let root = dir.path();

    // The single quotes in the RQL string are crucial
    let query = "contains:'value * 2'";
    let results = run_test_search(root, query)?;

    assert_eq!(results, vec!["src/special.txt"]);
    Ok(())
}

#[test]
fn test_query_with_escaped_quote() -> Result<()> {
    let dir = setup_test_project()?;
    let root = dir.path();

    // Using double quotes for the value allows it to contain a single quote.
    let query = "contains:\"user's settings\"";
    let results = run_test_search(root, query)?;

    assert_eq!(results, vec!["src/special.txt"]);
    Ok(())
}
