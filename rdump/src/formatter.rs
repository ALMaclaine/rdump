use anyhow::{Result, Context};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Formats and prints the final output to a generic writer.
pub fn print_output(
    writer: &mut impl Write,
    matching_files: &[PathBuf],
    with_line_numbers: bool,
    no_headers: bool,
) -> Result<()> {
    if no_headers {
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
    } else {
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

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use std::path::PathBuf;

    fn create_temp_file_with_content(content: &str) -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_print_output_default() {
        let file1 = create_temp_file_with_content("line 1\nline 2");
        let file2 = create_temp_file_with_content("hello");
        let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];

        let mut writer = Vec::new();
        // CORRECTED: Added &mut writer
        print_output(&mut writer, &paths, false, false).unwrap();

        let output = String::from_utf8(writer).unwrap();
        let expected = format!(
            "File: {}\n---\nline 1\nline 2\n\n---\n\nFile: {}\n---\nhello\n",
            file1.path().display(),
            file2.path().display()
        );
        assert_eq!(output, expected);
    }

    #[test]
    fn test_print_output_with_line_numbers() {
        let file = create_temp_file_with_content("a\nb");
        let paths = vec![file.path().to_path_buf()];

        let mut writer = Vec::new();
        // CORRECTED: Added &mut writer
        print_output(&mut writer, &paths, true, false).unwrap();

        let output = String::from_utf8(writer).unwrap();
        let expected = format!(
            "File: {}\n---\n    1 | a\n    2 | b\n",
            file.path().display()
        );
        assert_eq!(output, expected);
    }

    #[test]
    fn test_print_output_no_headers() {
        let file1 = create_temp_file_with_content("content1");
        let file2 = create_temp_file_with_content("content2");
        let paths = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];

        let mut writer = Vec::new();
        // CORRECTED: Added &mut writer
        print_output(&mut writer, &paths, false, true).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output, "content1\ncontent2\n");
    }

    #[test]
    fn test_print_output_no_headers_with_line_numbers() {
        let file = create_temp_file_with_content("a\nb");
        let paths = vec![file.path().to_path_buf()];

        let mut writer = Vec::new();
        // CORRECTED: Added &mut writer
        print_output(&mut writer, &paths, true, true).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output, "    1 | a\n    2 | b\n");
    }

    #[test]
    fn test_print_output_no_matches() {
        let paths: Vec<PathBuf> = vec![];
        let mut writer = Vec::new();
        // CORRECTED: Added &mut writer
        print_output(&mut writer, &paths, false, false).unwrap();
        let output = String::from_utf8(writer).unwrap();
        assert_eq!(output, "");
    }
}