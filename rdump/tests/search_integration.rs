use anyhow::Result;
use rdump::{commands::search::perform_search, ColorChoice, Format, SearchArgs};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Helper to create a default SearchArgs for testing.
/// We enable `no_ignore` and `hidden` to make tests self-contained and predictable.
fn create_test_args(root: &Path, query: &str) -> SearchArgs {
    SearchArgs {
        query: vec![query.to_string()],
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
///
/// /src/user.rs        (struct User, // TODO)
/// /src/order.rs       (struct Order)
/// /tests/user_test.rs (fn test_user)
/// /benches/user.rs    (fn bench_user)
/// /docs/api.md        (API Docs)
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
