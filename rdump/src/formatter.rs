use anyhow::{Context, Result};
use chrono::{DateTime, Local}; // For formatting timestamps
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::ops::Range as StdRange;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt; // For Unix permissions
use std::path::PathBuf;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use tree_sitter::Range;

// We need to pass the format enum from main.rs
use crate::Format;

// Lazily load syntax and theme sets once.
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct FileOutput {
    path: String,
    content: String,
}

fn print_markdown_format(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
    with_line_numbers: bool,
    use_color: bool,
) -> Result<()> {
    for (i, (path, _)) in matching_files.iter().enumerate() {
        if i > 0 {
            writeln!(writer, "\n---\n")?;
        }
        writeln!(writer, "File: {}", path.display())?;
        writeln!(writer, "---")?;
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        if use_color {
            // To terminal: use ANSI codes for color
            print_highlighted_content(writer, &content, extension, with_line_numbers)?;
        } else {
            // To file/pipe: use Markdown fences for color
            print_markdown_fenced_content(writer, &content, extension, with_line_numbers)?;
        }
    }
    Ok(())
}

fn print_cat_format(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
    with_line_numbers: bool,
    use_color: bool,
) -> Result<()> {
    for (path, _) in matching_files {
        let content = fs::read_to_string(path)?;
        if use_color {
            // To terminal
            print_highlighted_content(
                writer,
                &content,
                &path.extension().and_then(|s| s.to_str()).unwrap_or(""),
                with_line_numbers,
            )?;
        } else {
            print_plain_content(writer, &content, with_line_numbers)?; // To file/pipe
        }
    }
    Ok(())
}

fn print_json_format(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
) -> Result<()> {
    let mut outputs = Vec::new();
    for (path, _) in matching_files {
        let content = fs::read_to_string(path).with_context(|| {
            format!("Failed to read file for final output: {}", path.display())
        })?;
        outputs.push(FileOutput {
            path: path.to_string_lossy().to_string(),
            content,
        });
    }
    // Use to_writer_pretty for readable JSON output
    serde_json::to_writer_pretty(writer, &outputs)?;
    Ok(())
}

fn print_paths_format(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
) -> Result<()> {
    for (path, _) in matching_files {
        writeln!(writer, "{}", path.display())?;
    }
    Ok(())
}

fn print_find_format(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
) -> Result<()> {
    for (path, _) in matching_files {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for {}", path.display()))?;
        let size = metadata.len();
        let modified: DateTime<Local> = DateTime::from(metadata.modified()?);

        // Get permissions (basic implementation)
        let perms = metadata.permissions();
        #[cfg(unix)]
        let mode = perms.mode();
        #[cfg(not(unix))]
        let mode = 0; // Placeholder for non-unix
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
    Ok(())
}

fn print_hunks_format(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
    with_line_numbers: bool,
    use_color: bool,
    context_lines: usize,
) -> Result<()> {
    for (i, (path, hunks)) in matching_files.iter().enumerate() {
        if i > 0 {
            writeln!(writer, "\n---\n")?;
        }
        writeln!(writer, "File: {}", path.display())?;
        writeln!(writer, "---")?;
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        if hunks.is_empty() {
            // Boolean match, print the whole file
            print_content_with_style(writer, &content, extension, with_line_numbers, use_color)?;
        } else {
            // Hunk match, print with context
            let lines: Vec<&str> = content.lines().collect();
            let line_ranges = get_contextual_line_ranges(hunks, &lines, context_lines);

            for (i, range) in line_ranges.iter().enumerate() {
                if i > 0 {
                    writeln!(writer, "...")?;
                }
                writeln!(writer, "```{}", extension)?;
                for line_num in range.clone() {
                    if let Some(line) = lines.get(line_num) {
                        if with_line_numbers {
                            write!(writer, "{: >5} | ", line_num + 1)?;
                        }
                        writeln!(writer, "{}", line)?;
                    }
                }
                writeln!(writer, "```")?;
            }
        }
    }
    Ok(())
}

/// Formats and prints the final output to a generic writer based on the chosen format.
pub fn print_output(
    writer: &mut impl Write,
    matching_files: &[(PathBuf, Vec<Range>)],
    format: &Format,
    with_line_numbers: bool,
    use_color: bool,
    context_lines: usize,
) -> Result<()> {
    match format {
        Format::Find => print_find_format(writer, matching_files)?,
        Format::Paths => print_paths_format(writer, matching_files)?,
        Format::Json => print_json_format(writer, matching_files)?,
        Format::Cat => print_cat_format(writer, matching_files, with_line_numbers, use_color)?,
        Format::Markdown => {
            print_markdown_format(writer, matching_files, with_line_numbers, use_color)?
        }
        Format::Hunks => print_hunks_format(
            writer,
            matching_files,
            with_line_numbers,
            use_color,
            context_lines,
        )?,
    }
    Ok(())
}



/// Helper to choose the correct printing function based on color/style preference.
fn print_content_with_style(
    writer: &mut impl Write,
    content: &str,
    extension: &str,
    with_line_numbers: bool,
    use_color: bool,
) -> Result<()> {
    if use_color {
        print_highlighted_content(writer, content, extension, with_line_numbers)
    } else {
        print_markdown_fenced_content(writer, content, extension, with_line_numbers)
    }
}

/// Given a set of byte-offset ranges, calculate the line number ranges including context,
/// and merge any overlapping ranges.
fn get_contextual_line_ranges(
    hunks: &[Range],
    lines: &[&str],
    context_lines: usize,
) -> Vec<StdRange<usize>> {
    if hunks.is_empty() || lines.is_empty() {
        return vec![];
    }

    let mut line_ranges = Vec::new();
    for hunk in hunks {
        let start_line = hunk.start_point.row;
        let end_line = hunk.end_point.row;

        let context_start = start_line.saturating_sub(context_lines);
        let context_end = (end_line + context_lines).min(lines.len() - 1);

        if context_end >= context_start {
            line_ranges.push(context_start..context_end + 1);
        }
    }
    line_ranges.sort_by_key(|r| r.start);

    // Merge overlapping ranges
    let mut merged_ranges = Vec::new();
    let mut iter = line_ranges.into_iter();
    if let Some(mut current) = iter.next() {
        for next in iter {
            if next.start <= current.end {
                current.end = current.end.max(next.end);
            } else {
                merged_ranges.push(current);
                current = next;
            }
        }
        merged_ranges.push(current);
    }
    merged_ranges
}


/// Prints syntax-highlighted content to the writer.
fn print_highlighted_content(
    writer: &mut impl Write,
    content: &str,
    extension: &str,
    with_line_numbers: bool,
) -> Result<()> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    for (i, line) in LinesWithEndings::from(content).enumerate() {
        if with_line_numbers {
            write!(writer, "{: >5} | ", i + 1)?;
        }
        let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &SYNTAX_SET)?;
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        write!(writer, "{}", escaped)?;
    }
    // Reset color at the end
    write!(writer, "\x1b[0m")?;
    Ok(())
}

/// Prints plain content, optionally with line numbers.
fn print_plain_content(
    writer: &mut impl Write,
    content: &str,
    with_line_numbers: bool,
) -> Result<()> {
    for (i, line) in content.lines().enumerate() {
        if with_line_numbers {
            writeln!(writer, "{: >5} | {}", i + 1, line)?;
        } else {
            writeln!(writer, "{}", line)?;
        }
    }
    Ok(())
}

/// Prints content inside a Markdown code fence.
fn print_markdown_fenced_content(
    writer: &mut impl Write,
    content: &str,
    extension: &str,
    with_line_numbers: bool,
) -> Result<()> {
    writeln!(writer, "```{}", extension)?;
    // print_plain_content handles line numbers correctly
    print_plain_content(writer, content, with_line_numbers)?;
    writeln!(writer, "```")?;
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
        format!(
            "-{}{}{}{}{}{}{}{}{}",
            user_r, user_w, user_x, group_r, group_w, group_x, other_r, other_w, other_x
        )
    }
    #[cfg(not(unix))]
    {
        // Basic fallback for non-Unix platforms
        if mode & 0o200 != 0 {
            "-rw-------"
        } else {
            "-r--------"
        }
        .to_string()
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper to create a temp file with some content.
    fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_format_plain_cat_with_line_numbers() {
        let file = create_temp_file_with_content("a\nb");
        let paths = vec![(file.path().to_path_buf(), vec![])];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Cat, true, false, 0).unwrap();
        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output, "    1 | a\n    2 | b\n");
    }

    #[test]
    fn test_format_paths() {
        let file1 = create_temp_file_with_content("a");
        let file2 = create_temp_file_with_content("b");
        let paths = vec![
            (file1.path().to_path_buf(), vec![]),
            (file2.path().to_path_buf(), vec![]),
        ];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Paths, false, false, 0).unwrap();
        let output = String::from_utf8(writer).unwrap();
        let expected = format!("{}\n{}\n", file1.path().display(), file2.path().display());
        assert_eq!(output, expected);
    }

    #[test]
    fn test_format_markdown_with_fences() {
        let file = create_temp_file_with_content("line 1");
        let paths = vec![(file.path().to_path_buf(), vec![])];
        let mut writer = Vec::new();

        // Test with use_color = false to get markdown fences
        print_output(&mut writer, &paths, &Format::Markdown, false, false, 0).unwrap();

        let output = String::from_utf8(writer).unwrap();

        let expected_header = format!("File: {}\n---\n", file.path().display());
        assert!(output.starts_with(&expected_header));
        // The extension of a tempfile is random, so we check for an empty language hint
        assert!(output.contains("```\nline 1\n```"));
    }

    #[test]
    fn test_format_markdown_with_ansi_color() {
        let file = create_temp_file_with_content("fn main() {}");
        // Give it a .rs extension so syntect can find the grammar
        let rs_path = file.path().with_extension("rs");
        std::fs::rename(file.path(), &rs_path).unwrap();

        let paths = vec![(rs_path, vec![])];
        let mut writer = Vec::new();
        print_output(&mut writer, &paths, &Format::Cat, false, true, 0).unwrap();
        let output = String::from_utf8(writer).unwrap();

        // Check for evidence of ANSI color, not the exact codes which can be brittle.
        assert!(
            output.contains("\x1b["),
            "Should contain ANSI escape codes"
        );
        assert!(!output.contains("```"), "Should not contain markdown fences");
    }
}
