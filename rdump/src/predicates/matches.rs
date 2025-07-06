use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct MatchesEvaluator;
impl PredicateEvaluator for MatchesEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let content = context.get_content()?;
        let re = regex::Regex::new(value)?;
        Ok(MatchResult::Boolean(re.is_match(content)))
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
    fn test_matches_evaluator() {
        let file = create_temp_file("version = \"0.1.0\"\nauthor = \"test\"");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = MatchesEvaluator;
        // Simple regex
        assert!(evaluator
            .evaluate(
                &mut context,
                &PredicateKey::Matches,
                r#"version = "[0-9]+\.[0-9]+\.[0-9]+""#
            )
            .unwrap()
            .is_match());
        // Test regex that spans lines
        assert!(evaluator
            .evaluate(
                &mut context,
                &PredicateKey::Matches,
                r#"(?s)version.*author"#
            )
            .unwrap()
            .is_match());
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::Matches,
                r#"^version = "1.0.0"$"#
            )
            .unwrap()
            .is_match());
    }
}
