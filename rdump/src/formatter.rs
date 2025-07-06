use anyhow::{Context, Result};
use chrono::{DateTime, Local}; // For formatting timestamps
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt; // For Unix permissions
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

// We need to pass the format enum from main.rs
use crate::Format;

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
) -> Result<()> {
    match format {
        Format::Find => {
            for path in matching_files {
                let metadata = fs::metadata(path)?;
                let size = metadata.len();
                let modified: DateTime<Local> = DateTime::from(metadata.modified()?);

                // Get permissions (basic implementation)
                let perms = metadata.permissions();
                let mode = perms.mode();
                let perms_str = format_mode(mode);

                // Format size into human-readable string
                let size_str = format_size(size);

                // Format time
                let time_str = modified.format("%b %d %H:%M").to_string();

                writeln!(
                    writer,
                    "{:<12} {:>8} {} {}",
                    perms_str,
                    size_str,
                    time_str,
                    path.display()
                )?;
            }
        }
        Format::Paths => {
            for path in matching_files {
                writeln!(writer, "{}", path.display())?;
            }
        }
        Format::Cat => {
            for path in matching_files {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file for final output: {}", path.display()))?;
                if with_line_numbers {
                    for (i, line) in content.lines().enumerate() {
                        writeln!(writer, "{:>5} | {}", i + 1, line)?;
                    }
                } else {
                    writeln!(writer, "{}", content)?;
                }
            }
        }
        Format::Json => {
            let mut outputs = Vec::new();
            for path in matching_files {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file for final output: {}", path.display()))?;
                outputs.push(FileOutput {
                    path: path.to_string_lossy().to_string(),
                    content,
                });
            }
            // Use to_writer_pretty for readable JSON output
            serde_json::to_writer_pretty(writer, &outputs)?;
        }
        Format::Markdown => {
            for (i, path) in matching_files.iter().enumerate() {
                if i > 0 {
                    writeln!(writer, "\n---\n")?;
                }
                writeln!(writer, "File: {}", path.display())?;
                writeln!(writer, "---")?;
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file for final output: {}", path.display()))?;

                if with_line_numbers {
                    for (i, line) in content.lines().enumerate() {
                        writeln!(writer, "{:>5} | {}", i + 1, line)?;
                    }
                } else {
                    writeln!(writer, "{}", content)?;
                }
            }
        }
    }
    Ok(())
}

fn format_mode(mode: u32) -> String {
    #[cfg(unix)]
    {
        let user_r = if mode & 0o400 != 0 { 'r' } else { '-' };
        let user_w = if mode & 0o200 != 0 { 'w' } else { '-' };
        let user_x = if mode & 0o100 != 0 { 'x' } else { '-' };
        let group_r = if mode & 0o040 != 0 { 'r' } else { '-' };
        let group_w = if mode & 0o020 != 0 { 'w' } else { '-' };
        let group_x = if mode & 0o010 != 0 { 'x' } else { '-' };
        let other_r = if mode & 0o004 != 0 { 'r' } else { '-' };
        let other_w = if mode & 0o002 != 0 { 'w' } else { '-' };
        let other_x = if mode & 0o001 != 0 { 'x' } else { '-' };
        format!("-{}{}{}{}{}{}{}{}{}", user_r, user_w, user_x, group_r, group_w, group_x, other_r, other_w, other_x)
    }
    #[cfg(not(unix))]
    {
        // Basic fallback for non-Unix platforms
        if mode & 0o200 != 0 { "-rw-------" } else { "-r--------" }.to_string()
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    // --- UPDATED AND NEW TESTS ---

    #[test]
    fn test_format_markdown() {
        let file = create_temp_file_with_content("line 1");
        let paths = vec![file.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Markdown, false).unwrap();
        let output = String::from_utf8(writer).unwrap();
        let expected = format!("File: {}\n---\nline 1\n", file.path().display());
        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_cat_with_line_numbers() {
        let file = create_temp_file_with_content("a\nb");
        let paths = vec![file.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Cat, true).unwrap();
        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output, "    1 | a\n    2 | b\n");
    }

    #[test]
    fn test_format_paths() {
        let file1 = create_temp_file_with_content("a");
        let file2 = create_temp_file_with_content("b");
        let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Paths, false).unwrap();
        let output = String::from_utf8(writer).unwrap();
        let expected = format!("{}\n{}\n", file1.path().display(), file2.path().display());
        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_json() {
        let file1 = create_temp_file_with_content("{\"key\": \"value\"}");
        let file2 = create_temp_file_with_content("some text");
        let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Json, false).unwrap();

        // The output is pretty-printed, so we compare the parsed data, not the raw string.
        let output_data: Vec<FileOutput> = serde_json::from_slice(&writer).unwrap();

        assert_eq!(output_data.len(), 2);
        assert_eq!(output_data[0].path, file1.path().to_string_lossy());
        assert_eq!(output_data[0].content, "{\"key\": \"value\"}");
        assert_eq!(output_data[1].path, file2.path().to_string_lossy());
        assert_eq!(output_data[1].content, "some text");
    }

    #[test]
    fn test_format_find() {
        let file = create_temp_file_with_content("hello"); // 5 bytes
        let paths = vec![file.path().to_path_buf()];
        let mut writer = Vec::new();

        print_output(&mut writer, &paths, &Format::Find, false).unwrap();

        let output = String::from_utf8(writer).unwrap();

        // We can't test the exact permissions or timestamp, but we can test the structure.
        assert!(output.contains("5B")); // Check for size
        assert!(output.contains(file.path().to_str().unwrap())); // Check for path
        assert!(output.ends_with('\n'));
    }
}
