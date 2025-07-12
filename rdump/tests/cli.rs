// In rdump/tests/cli.rs

use assert_cmd::prelude::*; // Adds methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::fs;
use std::io::Write;
use std::process::Command; // Lets us run other programs
use tempfile::tempdir; // Create temporary directories for testing

// --- Helper Functions ---

/// A helper to set up a temporary directory with a predictable file structure for tests.
/// Returns the TempDir object (which cleans up the directory when it's dropped)
/// and the PathBuf of the root, for convenience.
fn setup_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // Create a file to be found
    fs::File::create(root.join("main.rs"))
        .unwrap()
        .write_all(b"fn main() {\n    println!(\"Hello\");\n}")
        .unwrap();

    // Create a file that shouldn't be found by most queries
    fs::File::create(root.join("other.txt"))
        .unwrap()
        .write_all(b"some text")
        .unwrap();

    (dir, root)
}

// --- Test Implementation ---

#[test]
fn test_help_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "A fast, expressive, code-aware tool",
        ))
        .stdout(predicate::str::contains("Usage: rdump <COMMAND>"))
        .stdout(predicate::str::contains("Commands:\n  search"))
        .stdout(predicate::str::contains("  preset"))
        .stdout(predicate::str::contains("Options:\n  -h, --help"))
        .stdout(predicate::str::contains("  -V, --version"));
    Ok(())
}

#[test]
fn test_version_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("rdump 0.1.0")); // Assumes version in Cargo.toml
    Ok(())
}

#[test]
fn test_no_args_fails() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.assert()
        .failure() // Should fail because a subcommand is required
        .stderr(predicate::str::contains("Usage: rdump <COMMAND>"));
    Ok(())
}

#[test]
fn test_search_simple_predicate_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    // Setup a temporary directory with our test files
    let (_dir, root) = setup_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root); // Run the command *from* our test directory
    cmd.arg("search").arg("ext:rs");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("File: ./main.rs"))
        .stdout(predicate::str::contains("fn main()"))
        .stdout(predicate::str::contains("---").count(1))
        .stdout(predicate::str::contains("other.txt").not());
    Ok(())
}

#[test]
fn test_search_no_results() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    cmd.arg("search").arg("ext:java"); // No java files exist

    cmd.assert().success().stdout(predicate::str::is_empty()); // Nothing should be printed
    Ok(())
}

#[test]
fn test_search_invalid_query_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    cmd.arg("search").arg("ext:"); // Query with a missing value

    cmd.assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("expected value"));
    Ok(())
}

// Add this new helper function to rdump/tests/cli.rs

/// Sets up a more complex directory for testing discovery and formatting.
fn setup_advanced_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // Top-level files
    fs::File::create(root.join("main.rs"))
        .unwrap()
        .write_all(b"fn main() {}")
        .unwrap();
    fs::File::create(root.join(".hidden_config"))
        .unwrap()
        .write_all(b"secret=true")
        .unwrap();

    // Subdirectory with a file
    fs::create_dir(root.join("src")).unwrap();
    fs::File::create(root.join("src/lib.rs"))
        .unwrap()
        .write_all(b"// a library")
        .unwrap();

    // Directory and file to be ignored
    fs::create_dir(root.join("logs")).unwrap();
    fs::File::create(root.join("logs/latest.log"))
        .unwrap()
        .write_all(b"INFO: started")
        .unwrap();

    // .gitignore file to ignore the logs
    let mut gitignore = fs::File::create(root.join(".gitignore")).unwrap();
    writeln!(gitignore, "*.log").unwrap();

    (dir, root)
}

// Add these new tests to the end of rdump/tests/cli.rs

#[test]
fn test_file_discovery_flags() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_advanced_test_dir();

    // Test --hidden flag
    let mut cmd_hidden = Command::cargo_bin("rdump")?;
    cmd_hidden.current_dir(&root);
    cmd_hidden.arg("search").arg("--hidden").arg("path:hidden");
    cmd_hidden
        .assert()
        .success()
        .stdout(predicate::str::contains(".hidden_config"));

    // Test --no-ignore flag
    let mut cmd_no_ignore = Command::cargo_bin("rdump")?;
    cmd_no_ignore.current_dir(&root);
    cmd_no_ignore
        .arg("search")
        .arg("--no-ignore")
        .arg("ext:log");
    cmd_no_ignore
        .assert()
        .success()
        .stdout(predicate::str::contains("latest.log"));

    // Test --max-depth flag
    let mut cmd_depth = Command::cargo_bin("rdump")?;
    cmd_depth.current_dir(&root);
    // This should find `main.rs` but not `src/lib.rs`
    cmd_depth
        .arg("search")
        .arg("--max-depth")
        .arg("1")
        .arg("ext:rs");
    cmd_depth
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("src/lib.rs").not());

    Ok(())
}

#[test]
fn test_output_formatting_flags() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_advanced_test_dir();

    // Test --format paths
    let mut cmd_paths = Command::cargo_bin("rdump")?;
    cmd_paths.current_dir(&root);
    cmd_paths
        .arg("search")
        .arg("--format")
        .arg("paths")
        .arg("ext:rs");
    // Should contain ONLY the paths, sorted. The extra newline is important.
    let expected_paths = "./main.rs\n./src/lib.rs\n";
    cmd_paths.assert().success().stdout(expected_paths);

    // Test --format json
    let mut cmd_json = Command::cargo_bin("rdump")?;
    cmd_json.current_dir(&root);
    cmd_json
        .arg("search")
        .arg("--format")
        .arg("json")
        .arg("path:main.rs");
    // Assert it contains the key parts of a valid JSON output
    cmd_json
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""path": "./main.rs""#))
        .stdout(predicate::str::contains(r#""content": "fn main() {}""#));

    // Test --format cat with --line-numbers (no color, since it's a pipe)
    let mut cmd_cat = Command::cargo_bin("rdump")?;
    cmd_cat.current_dir(&root);
    cmd_cat
        .arg("search")
        .arg("--format")
        .arg("cat")
        .arg("--line-numbers")
        .arg("path:main.rs");
    cmd_cat
        .assert()
        .success()
        .stdout(predicate::str::contains("1 | fn main() {}"))
        .stdout(predicate::str::contains("\x1b[").not()); // Check for NO ANSI color codes

    // Test --color=always to force highlighting
    let mut cmd_color = Command::cargo_bin("rdump")?;
    cmd_color.current_dir(&root);
    cmd_color
        .arg("search")
        .arg("--color=always")
        .arg("path:main.rs");
    cmd_color
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[")); // Check FOR ANSI color codes

    // Test --find shorthand flag
    let mut cmd_find = Command::cargo_bin("rdump")?;
    cmd_find.current_dir(&root);
    cmd_find.arg("search").arg("--find").arg("path:main.rs");
    // We can't know the exact permissions/date, but we can check for the structure
    cmd_find
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs")) // The path
        .stdout(predicate::str::contains("12")); // The size (fn main() {} is 12 bytes)

    Ok(())
}

/// Sets up an environment for testing presets, with a fake home and project directory.
fn setup_preset_test_env() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let fake_home = dir.path().join("home");
    fs::create_dir(&fake_home).unwrap();
    let project_dir = dir.path().join("project");
    fs::create_dir(&project_dir).unwrap();

    // Create a file in the project directory to be found by searches
    fs::File::create(project_dir.join("main.rs"))
        .unwrap()
        .write_all(b"fn main() {}")
        .unwrap();
    fs::File::create(project_dir.join("main.toml"))
        .unwrap()
        .write_all(b"[package]")
        .unwrap();

    (dir, fake_home, project_dir)
}

// Add these new tests to the end of rdump/tests/cli.rs

#[test]
fn test_preset_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // We get a clean home directory AND a clean project directory
    let (_dir, fake_home, project_dir) = setup_preset_test_env();
    let config_path = fake_home.join("rdump/config.toml");

    // 1. List when no presets exist.
    // CRITICAL FIX: Run this command from the clean project_dir.
    let mut cmd_list1 = Command::cargo_bin("rdump")?;
    cmd_list1.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_list1.current_dir(&project_dir); // <--- THIS IS THE FIX
    cmd_list1.arg("preset").arg("list");
    cmd_list1
        .assert()
        .success()
        .stdout(predicate::str::contains("No presets found."));

    // 2. Add a preset. This command is not affected by current_dir, but it's good practice.
    let mut cmd_add = Command::cargo_bin("rdump")?;
    cmd_add.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_add.current_dir(&project_dir); // Add for consistency
    cmd_add
        .arg("preset")
        .arg("add")
        .arg("rust-files")
        .arg("ext:rs");
    cmd_add.assert().success();

    // Verify the file content directly
    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path)?;
    assert!(content.contains(r#"rust-files = "ext:rs""#));

    // 3. List again to see the new preset.
    let mut cmd_list2 = Command::cargo_bin("rdump")?;
    cmd_list2.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_list2.current_dir(&project_dir); // Add for consistency
    cmd_list2.arg("preset").arg("list");
    cmd_list2
        .assert()
        .success()
        .stdout(predicate::str::contains("rust-files : ext:rs"));

    // 4. Remove the preset.
    let mut cmd_remove = Command::cargo_bin("rdump")?;
    cmd_remove.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_remove.current_dir(&project_dir); // Add for consistency
    cmd_remove.arg("preset").arg("remove").arg("rust-files");
    cmd_remove.assert().success();

    // Verify the file content has changed
    let content_after_remove = fs::read_to_string(&config_path)?;
    assert!(!content_after_remove.contains("rust-files"));

    Ok(())
}

#[test]
fn test_search_and_preset_interaction() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, fake_home, project_dir) = setup_preset_test_env();

    // 1. Setup: Create presets
    let preset_content = r#"
[presets]
rust = "ext:rs"
config = "ext:toml"
"#;
    let config_dir = fake_home.join("rdump");
    fs::create_dir_all(&config_dir)?;
    fs::write(config_dir.join("config.toml"), preset_content)?;

    // 2. Test search with one preset
    let mut cmd_search1 = Command::cargo_bin("rdump")?;
    cmd_search1.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_search1.current_dir(&project_dir);
    cmd_search1.arg("search").arg("-p").arg("rust");
    cmd_search1
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("main.toml").not());

    // 3. Test search with a preset AND a query
    // Should evaluate to `(ext:rs) & contains:main`
    let mut cmd_search2 = Command::cargo_bin("rdump")?;
    cmd_search2.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_search2.current_dir(&project_dir);
    cmd_search2
        .arg("search")
        .arg("-p")
        .arg("rust")
        .arg("contains:main");
    cmd_search2
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs"));

    // 4. Test search with multiple presets
    // Should evaluate to `(ext:toml) & (ext:rs)` -> no results
    let mut cmd_search3 = Command::cargo_bin("rdump")?;
    cmd_search3.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd_search3.current_dir(&project_dir);
    cmd_search3
        .arg("search")
        .arg("-p")
        .arg("rust")
        .arg("-p")
        .arg("config");
    cmd_search3
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    Ok(())
}

#[test]
fn test_local_config_override() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, fake_home, project_dir) = setup_preset_test_env();

    // 1. Setup: Create a global config
    let global_preset = r#"[presets]
app = "ext:rs""#;
    let config_dir = fake_home.join("rdump");
    fs::create_dir_all(&config_dir)?;
    fs::write(config_dir.join("config.toml"), global_preset)?;

    // 2. Setup: Create a local .rdump.toml that OVERRIDES the preset
    let local_preset = r#"[presets]
app = "ext:toml""#;
    fs::write(project_dir.join(".rdump.toml"), local_preset)?;

    // 3. Run the search from the project directory
    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    cmd.current_dir(&project_dir); // CRITICAL: Run from where the local config is
    cmd.arg("search").arg("-p").arg("app");

    // 4. Assert that it used the LOCAL definition (ext:toml) and not the global one (ext:rs)
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.toml"))
        .stdout(predicate::str::contains("main.rs").not());

    Ok(())
}

#[test]
fn test_preset_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, fake_home, _project_dir) = setup_preset_test_env();

    // Test removing a preset that doesn't exist
    let mut cmd_remove = Command::cargo_bin("rdump")?;
    cmd_remove.env("RDUMP_TEST_CONFIG_DIR", &fake_home);
    // Note: We have to create an empty config file first for the remove error to trigger
    let config_dir = fake_home.join("rdump");
    fs::create_dir_all(&config_dir)?;
    fs::write(config_dir.join("config.toml"), "")?;
    cmd_remove.arg("preset").arg("remove").arg("no-such-preset");

    cmd_remove
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Preset 'no-such-preset' not found",
        ));

    Ok(())
}

#[test]
fn test_search_with_or_operator() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    cmd.arg("search").arg("ext:rs | contains:text");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("other.txt"));
    Ok(())
}
