use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;
use tree_sitter::Range;

pub(super) struct ContainsEvaluator;

impl PredicateEvaluator for ContainsEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let content = context.get_content()?;
        let mut ranges = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.contains(value) {
                let start_byte = content.lines().take(i).map(|l| l.len() + 1).sum();
                let end_byte = start_byte + line.len();
                let range = Range {
                    start_byte,
                    end_byte,
                    start_point: tree_sitter::Point { row: i, column: 0 },
                    end_point: tree_sitter::Point {
                        row: i,
                        column: line.len(),
                    },
                };
                ranges.push(range);
            }
        }
        Ok(MatchResult::Hunks(ranges))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }
    #[test]
    fn test_contains_evaluator() {
        let file = create_temp_file("Hello world\nThis is a test.");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = ContainsEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "world")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "is a test")
            .unwrap()
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "goodbye")
            .unwrap()
            .is_match());
    }
}
