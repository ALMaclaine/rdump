use anyhow::Result;
use std::path::PathBuf;

use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;

pub(super) struct InPathEvaluator;

impl PredicateEvaluator for InPathEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let target_dir = PathBuf::from(value);
        let absolute_target_dir = if target_dir.is_absolute() {
            target_dir
        } else {
            context.root.join(target_dir)
        };

        // If the target directory doesn't exist, it can't contain any files.
        if !absolute_target_dir.exists() {
            return Ok(MatchResult::Boolean(false));
        }

        // Canonicalize both paths to resolve any `.` or `..` components.
        // This ensures a reliable comparison.
        let canonical_target = dunce::canonicalize(&absolute_target_dir)?;
        let canonical_file_path = dunce::canonicalize(&context.path)?;

        Ok(MatchResult::Boolean(
            canonical_file_path.starts_with(&canonical_target),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_in_path_evaluator() -> Result<()> {
        let evaluator = InPathEvaluator;

        // Create a temporary directory structure
        let root_dir = tempdir()?;
        let root_path = root_dir.path();

        let project_dir = root_path.join("project");
        let src_dir = project_dir.join("src");
        let other_project_dir = root_path.join("other_project");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&other_project_dir)?;

        let main_rs_path = src_dir.join("main.rs");
        fs::write(&main_rs_path, "fn main() {}")?;

        // --- Test Cases ---

        let mut context = FileContext::new(main_rs_path.clone(), root_path.to_path_buf());

        // 1. Absolute Path: Exact parent directory
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, src_dir.to_str().unwrap())?
            .is_match());

        // 2. Absolute Path: Grandparent directory
        assert!(evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                project_dir.to_str().unwrap()
            )?
            .is_match());

        // 3. Absolute Path: Non-matching directory
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                other_project_dir.to_str().unwrap()
            )?
            .is_match());

        // 4. Relative Path: from the root
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, "project/src")?
            .is_match());

        // 5. Relative Path: with dot-slash
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, "./project/src")?
            .is_match());

        // 6. Relative Path: non-matching
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::In, "other_project")?
            .is_match());
            
        // 7. A file is considered to be "in" the directory represented by its own path
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, main_rs_path.to_str().unwrap())?
            .is_match());

        // 8. Non-existent directory should not error, just return false
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::In, "non_existent_dir")?
            .is_match());

        Ok(())
    }
}

