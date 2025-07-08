use super::{helpers, PredicateEvaluator};
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct SizeEvaluator;
impl PredicateEvaluator for SizeEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let metadata = context.path.metadata()?;
        let file_size = metadata.len();
        Ok(MatchResult::Boolean(helpers::parse_and_compare_size(
            file_size, value,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_size_evaluator() {
        let file = create_temp_file("a".repeat(2000).as_str());
        let mut context = FileContext::new(file.path().to_path_buf(), PathBuf::from("/"));

        let evaluator = SizeEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">1000")
            .unwrap()
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Size, "<1kb")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">0.9kb")
            .unwrap()
            .is_match());
    }
}
