use anyhow::{Context, Result};
use chrono::{DateTime, Local}; // For formatting timestamps
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt; // For Unix permissions
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// We need to pass the format enum from main.rs
use crate::Format;

lazy_static! {
    // Lazily load syntax and theme sets once.
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct FileOutput {
    path: String,
    content: String,
}

/// Formats and prints the final output to a generic writer based on the chosen format.
pub fn print_output(
    writer: &mut impl Write,
    matching_files: &[PathBuf],
    format: &Format,
    with_line_numbers: bool,
    use_color: bool,
) -> Result<()> {
    match format {
        Format::Find => {
            for path in matching_files {
                let metadata = fs::metadata(path)
                    .with_context(|| format!("Failed to read metadata for {}", path.display()))?;

                let file_size = metadata.len();
                let permissions = metadata.permissions();
                let mode = permissions.mode();
                let modified_time: DateTime<Local> = metadata.modified()?.into();

                // Format the mode into a human-readable string (e.g., "drwxr-xr-x")
                let mode_str = format_mode(mode);

                // Format the output similar to `find . -ls`
                writeln!(
                    writer,
                    "{:>10} {:>5} {:>8} {} {}",
                    metadata.ino(),
                    file_size,
                    mode_str,
                    modified_time.format("%b %d %H:%M"),
                    path.display()
                )?;
            }
        }
        Format::Paths => {
            for path in matching_files {
                writeln!(writer, "{}", path.display())?;
            }
        }
        Format::Json => {
            let mut outputs = Vec::new();
            for path in matching_files {
                let content = fs::read_to_string(path).with_context(|| {
                    format!("Failed to read file for JSON output: {}", path.display())
                })?;
                outputs.push(FileOutput {
                    path: path.display().to_string(),
                    content,
                });
            }
            let json = serde_json::to_string_pretty(&outputs)
                .context("Failed to serialize output to JSON")?;
            writeln!(writer, "{}", json)?;
        }
        Format::Cat => {
            for path in matching_files {
                let content = fs::read_to_string(path)?;
                if use_color {
                    print_highlighted_content(
                        writer,
                        &content,
                        &path.extension().and_then(|s| s.to_str()).unwrap_or(""),
                        with_line_numbers,
                    )?;
                } else {
                    print_plain_content(writer, &content, with_line_numbers)?;
                }
            }
        }
        Format::Markdown => {
            for path in matching_files {
                if matching_files.len() > 1 {
                    writeln!(writer, "## {}", path.display())?;
                } else {
                    writeln!(writer, "File: {}", path.display())?;
                }
                writeln!(writer, "---")?;
                let content = fs::read_to_string(path)?;

                if use_color {
                    print_highlighted_content(
                        writer,
                        &content,
                        &path.extension().and_then(|s| s.to_str()).unwrap_or(""),
                        with_line_numbers,
                    )?;
                } else {
                    print_plain_content(writer, &content, with_line_numbers)?;
                }
            }
        }
    }
    Ok(())
}

/// Prints content with syntax highlighting.
fn print_highlighted_content(
    writer: &mut impl Write,
    content: &str,
    extension: &str,
    with_line_numbers: bool,
) -> Result<()> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
    // Using a theme with a background color is important for correctness.
    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    for (i, line) in LinesWithEndings::from(content).enumerate() {
        if with_line_numbers {
            write!(writer, "{:>5} | ", i + 1)?;
        }
        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &SYNTAX_SET)?;
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        write!(writer, "{}", escaped)?;
    }
    // Reset terminal colors
    write!(writer, "\x1b[0m")?;
    Ok(())
}

/// Prints content without syntax highlighting.
fn print_plain_content(writer: &mut impl Write, content: &str, with_line_numbers: bool) -> Result<()> {
    for (i, line) in content.lines().enumerate() {
        if with_line_numbers {
            writeln!(writer, "{:>5} | {}", i + 1, line)?;
        } else {
            writeln!(writer, "{}", line)?;
        }
    }
    Ok(())
}

fn format_mode(mode: u32) -> String {
    #[cfg(unix)]
    {
        let mut perms = String::new();
        // File type
        if (mode & 0o170000) == 0o120000 {
            perms.push('l'); // Symbolic link
        } else if (mode & 0o170000) == 0o040000 {
            perms.push('d'); // Directory
        } else {
            perms.push('-'); // Regular file
        }
        // Owner permissions
        perms.push(if (mode & 0o400) != 0 { 'r' } else { '-' });
        perms.push(if (mode & 0o200) != 0 { 'w' } else { '-' });
        perms.push(if (mode & 0o100) != 0 { 'x' } else { '-' });
        // Group permissions
        perms.push(if (mode & 0o040) != 0 { 'r' } else { '-' });
        perms.push(if (mode & 0o020) != 0 { 'w' } else { '-' });
        perms.push(if (mode & 0o010) != 0 { 'x' } else { '-' });
        // Other permissions
        perms.push(if (mode & 0o004) != 0 { 'r' } else { '-' });
        perms.push(if (mode & 0o002) != 0 { 'w' } else { '-' });
        perms.push(if (mode & 0o001) != 0 { 'x' } else { '-' });
        perms
    }
    #[cfg(not(unix))]
    {
        // Basic mode formatting for non-Unix systems
        if (mode & 0o040000) != 0 {
            "d".to_string()
        } else {
            "-".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_format_markdown_single_file() {
        let file = create_temp_file_with_content("line 1");
        let paths = vec![file.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Markdown, false, false).unwrap();
        let output = String::from_utf8(writer).unwrap();
        let expected = format!("File: {}\n---\nline 1\n", file.path().display());
        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_cat_with_line_numbers() {
        let file = create_temp_file_with_content("a\nb");
        let paths = vec![file.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Cat, true, false).unwrap();
        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output, "    1 | a\n    2 | b\n");
    }

    #[test]
    fn test_format_paths() {
        let file1 = create_temp_file_with_content("a");
        let file2 = create_temp_file_with_content("b");
        let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Paths, false, false).unwrap();
        let output = String::from_utf8(writer).unwrap();
        let expected = format!("{}\n{}\n", file1.path().display(), file2.path().display());
        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_json() {
        let file1 = create_temp_file_with_content("hello");
        let file2 = create_temp_file_with_content("some text");
        let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Json, false, false).unwrap();

        // The output is pretty-printed, so we compare the parsed data, not the raw string.
        let output_data: Vec<FileOutput> = serde_json::from_slice(&writer).unwrap();

        let expected_data = vec![
            FileOutput {
                path: file1.path().display().to_string(),
                content: "hello".to_string(),
            },
            FileOutput {
                path: file2.path().display().to_string(),
                content: "some text".to_string(),
            },
        ];
        assert_eq!(output_data, expected_data);
    }

    #[test]
    fn test_format_find() {
        let file = create_temp_file_with_content("find-test");
        let paths = vec![file.path().to_path_buf()];
        let mut writer = Vec::new();

        print_output(&mut writer, &paths, &Format::Find, false, false).unwrap();

        let output = String::from_utf8(writer).unwrap();

        // Basic checks for the `find` format
        assert!(output.contains(file.path().to_str().unwrap())); // Check for path
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_highlighting_adds_ansi_codes() {
        let rust_code = "fn main() {}";
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, rust_code).unwrap();

        let paths = vec![file_path];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Cat, false, true).unwrap();
        let output = String::from_utf8(writer).unwrap();

        // A simple check to see if ANSI escape codes are present.
        assert!(output.contains("\x1b[38;2;"), "Output should contain ANSI color codes for highlighting");
        assert!(output.ends_with("\x1b[0m"), "Output should end with ANSI reset code");
    }
}