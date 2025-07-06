use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

// The core trait that all predicate evaluators must implement.
pub trait PredicateEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool>;
}

// --- Concrete Implementations ---

struct ExtEvaluator;
impl PredicateEvaluator for ExtEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let file_ext = context.path.extension().and_then(|s| s.to_str()).unwrap_or("");
        Ok(file_ext.eq_ignore_ascii_case(value))
    }
}

struct PathEvaluator;
impl PredicateEvaluator for PathEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let path_str = context.path.to_string_lossy();
        Ok(path_str.contains(value))
    }
}

struct NameEvaluator;
impl PredicateEvaluator for NameEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let file_name = context.path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let pattern = glob::Pattern::new(value)?;
        Ok(pattern.matches(file_name))
    }
}

struct ContainsEvaluator;
impl PredicateEvaluator for ContainsEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let content = context.get_content()?;
        Ok(content.contains(value))
    }
}

struct MatchesEvaluator;
impl PredicateEvaluator for MatchesEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let content = context.get_content()?;
        let re = regex::Regex::new(value)?;
        Ok(re.is_match(content))
    }
}

struct SizeEvaluator;
impl PredicateEvaluator for SizeEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let metadata = context.path.metadata()?;
        let file_size = metadata.len();
        parse_and_compare_size(file_size, value)
    }
}

struct ModifiedEvaluator;
impl PredicateEvaluator for ModifiedEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let metadata = context.path.metadata()?;
        let modified_time = metadata.modified()?;
        parse_and_compare_time(modified_time, value)
    }
}

// --- HELPER FUNCTIONS (moved from evaluator.rs) ---

fn parse_and_compare_size(file_size: u64, value: &str) -> Result<bool> {
    if value.len() < 2 {
        return Err(anyhow!("Invalid size format. Expected <op><num>[unit], e.g., '>10kb'"));
    }
    let op = value.chars().next().unwrap();
    let rest = &value[1..];
    let numeric_part_end = rest.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(rest.len());
    let (num_str, unit_str) = rest.split_at(numeric_part_end);
    let num: f64 = num_str.parse()?;
    let multiplier = match unit_str.trim().to_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        _ => return Err(anyhow!("Invalid size unit: '{}'", unit_str)),
    };
    let target_size = (num * multiplier) as u64;
    match op {
        '>' => Ok(file_size > target_size),
        '<' => Ok(file_size < target_size),
        _ => Err(anyhow!("Invalid size operator: '{}'", op)),
    }
}

fn parse_and_compare_time(modified_time: SystemTime, value: &str) -> Result<bool> {
    let (op, duration_str) = value.split_at(1);
    let now = SystemTime::now();
    let (num_str, unit) = duration_str.split_at(duration_str.len() - 1);
    let num: u64 = num_str.parse()?;
    let duration = match unit {
        "s" => Duration::from_secs(num),
        "m" => Duration::from_secs(num * 60),
        "h" => Duration::from_secs(num * 3600),
        "d" => Duration::from_secs(num * 3600 * 24),
        "w" => Duration::from_secs(num * 3600 * 24 * 7),
        _ => return Err(anyhow!("Invalid time unit: '{}'", unit)),
    };
    let cutoff_time = now - duration;
    match op {
        ">" => Ok(modified_time > cutoff_time),
        "<" => Ok(modified_time < cutoff_time),
        _ => Err(anyhow!("Invalid time operator: '{}'", op)),
    }
}

// The "Registry" that holds all our evaluators.
pub fn create_predicate_registry() -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
    let mut registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> = HashMap::new();

    // The `Send + Sync` bounds are required because we use this in a multi-threaded
    // context with Rayon. All our current evaluators are safe.
    registry.insert(PredicateKey::Ext, Box::new(ExtEvaluator));
    registry.insert(PredicateKey::Path, Box::new(PathEvaluator));
    registry.insert(PredicateKey::Name, Box::new(NameEvaluator));
    registry.insert(PredicateKey::Contains, Box::new(ContainsEvaluator));
    registry.insert(PredicateKey::Matches, Box::new(MatchesEvaluator));
    registry.insert(PredicateKey::Size, Box::new(SizeEvaluator));
    registry.insert(PredicateKey::Modified, Box::new(ModifiedEvaluator));

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::FileContext;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_size_evaluator() {
        let content: Vec<u8> = vec![0; 1024]; // Exactly 1 KB
        let file = create_temp_file("");
        file.as_file().write_all(&content).unwrap();
        let mut context = FileContext::new(file.path().to_path_buf());

        let evaluator = SizeEvaluator;
        assert!(evaluator.evaluate(&mut context, ">1000").unwrap());
        assert!(!evaluator.evaluate(&mut context, "<1kb").unwrap());
        assert!(evaluator.evaluate(&mut context, ">0.9kb").unwrap());
    }

    #[test]
    fn test_modified_evaluator() {
        let file = create_temp_file("content");
        let mut context = FileContext::new(file.path().to_path_buf());

        let evaluator = ModifiedEvaluator;
        // File was just created
        assert!(evaluator.evaluate(&mut context, ">1m").unwrap()); // Modified more recently than 1 min ago
        assert!(!evaluator.evaluate(&mut context, "<1m").unwrap()); // Not modified longer than 1 min ago
    }

    #[test]
    fn test_ext_evaluator() {
        let mut context_rs = FileContext::new(PathBuf::from("/tmp/test.rs"));
        let mut context_toml = FileContext::new(PathBuf::from("C:\\data\\Config.TOML"));
        let mut context_no_ext = FileContext::new(PathBuf::from("no_extension"));
        let mut context_dotfile = FileContext::new(PathBuf::from(".bashrc"));

        let evaluator = ExtEvaluator;
        assert!(evaluator.evaluate(&mut context_rs, "rs").unwrap());
        assert!(!evaluator.evaluate(&mut context_rs, "toml").unwrap());
        assert!(evaluator.evaluate(&mut context_toml, "toml").unwrap(), "Should be case-insensitive");
        assert!(!evaluator.evaluate(&mut context_no_ext, "rs").unwrap());
        assert!(!evaluator.evaluate(&mut context_dotfile, "bashrc").unwrap(), "Dotfiles should have no extension");
    }

    #[test]
    fn test_path_evaluator() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;
        assert!(evaluator.evaluate(&mut context, "project/src").unwrap());
        assert!(evaluator.evaluate(&mut context, "/home/user").unwrap());
        assert!(!evaluator.evaluate(&mut context, "project/lib").unwrap());
        assert!(evaluator.evaluate(&mut context, "main.rs").unwrap());
    }

    #[test]
    fn test_name_evaluator() {
        let mut context1 = FileContext::new(PathBuf::from("/home/user/Cargo.toml"));
        let mut context2 = FileContext::new(PathBuf::from("/home/user/main.rs"));

        let evaluator = NameEvaluator;
        assert!(evaluator.evaluate(&mut context1, "Cargo.toml").unwrap());
        assert!(evaluator.evaluate(&mut context1, "C*.toml").unwrap(), "Glob pattern should match");
        assert!(evaluator.evaluate(&mut context2, "*.rs").unwrap(), "Glob pattern should match");
        assert!(!evaluator.evaluate(&mut context1, "*.rs").unwrap());
    }

    #[test]
    fn test_contains_evaluator() {
        let file = create_temp_file("Hello world\nThis is a test.");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = ContainsEvaluator;
        assert!(evaluator.evaluate(&mut context, "world").unwrap());
        assert!(evaluator.evaluate(&mut context, "is a test").unwrap());
        assert!(!evaluator.evaluate(&mut context, "goodbye").unwrap());
    }

    #[test]
    fn test_matches_evaluator() {
        let file = create_temp_file("version = \"0.1.0\"\nauthor = \"test\"");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = MatchesEvaluator;
        // Simple regex
        assert!(evaluator.evaluate(&mut context, r#"version = "[0-9]+\.[0-9]+\.[0-9]+""#).unwrap());
        // Test regex that spans lines
        assert!(evaluator.evaluate(&mut context, r#"(?s)version.*author"#).unwrap());
        assert!(!evaluator.evaluate(&mut context, r#"^version = "1.0.0"$"#).unwrap());
    }
}
