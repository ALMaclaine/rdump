use anyhow::Result;
use globset::Glob;
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
        // Check for glob metacharacters to switch between logic paths.
        if value.contains('*') || value.contains('?') || value.contains('[') || value.contains('{')
        {
            // --- Wildcard Logic ---
            let glob = Glob::new(value)?.compile_matcher();

            if let Some(parent) = context.path.parent() {
                // Strip the root from the parent path to make the match relative.
                let relative_parent = parent.strip_prefix(&context.root).unwrap_or(parent);
                Ok(MatchResult::Boolean(glob.is_match(relative_parent)))
            } else {
                Ok(MatchResult::Boolean(false))
            }
        } else {
            // --- Non-recursive Exact-Path Logic ---
            let target_dir = PathBuf::from(value);
            let absolute_target_dir = if target_dir.is_absolute() {
                target_dir
            } else {
                context.root.join(target_dir)
            };

            if !absolute_target_dir.is_dir() {
                return Ok(MatchResult::Boolean(false));
            }

            let canonical_target = match dunce::canonicalize(&absolute_target_dir) {
                Ok(path) => path,
                Err(_) => return Ok(MatchResult::Boolean(false)),
            };

            if let Some(file_parent) = context.path.parent() {
                let canonical_file_parent = match dunce::canonicalize(file_parent) {
                    Ok(path) => path,
                    Err(_) => return Ok(MatchResult::Boolean(false)),
                };
                Ok(MatchResult::Boolean(canonical_file_parent == canonical_target))
            } else {
                Ok(MatchResult::Boolean(false))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_in_path_evaluator_exact() -> Result<()> {
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

        // 1. Absolute Path: Exact parent directory. Should match.
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, src_dir.to_str().unwrap())?
            .is_match());

        // 2. Absolute Path: Grandparent directory. Should NOT match due to non-recursive logic.
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                project_dir.to_str().unwrap()
            )?
            .is_match());

        // 3. Absolute Path: Non-matching directory.
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                other_project_dir.to_str().unwrap()
            )?
            .is_match());

        // 4. Relative Path: from the root. Should match.
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, "project/src")?
            .is_match());

        // 5. Relative Path: with dot-slash. Should match.
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, "./project/src")?
            .is_match());

        // 6. Relative Path: non-matching.
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::In, "other_project")?
            .is_match());

        // 7. A file path is not a directory. Should not match.
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                main_rs_path.to_str().unwrap()
            )?
            .is_match());

        // 8. Non-existent directory should not error, just return false.
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::In, "non_existent_dir")?
            .is_match());

        Ok(())
    }

    #[test]
    fn test_in_path_evaluator_wildcard() -> Result<()> {
        let evaluator = InPathEvaluator;

        // Create a temporary directory structure
        let root_dir = tempdir()?;
        let root_path = root_dir.path();

        let project_a_src = root_path.join("project_a").join("src");
        let project_b_source = root_path.join("project_b").join("source");
        let other_src = root_path.join("other").join("src");
        fs::create_dir_all(&project_a_src)?;
        fs::create_dir_all(&project_b_source)?;
        fs::create_dir_all(&other_src)?;

        let file_a = project_a_src.join("main.rs");
        fs::write(&file_a, "")?;
        let file_b = project_b_source.join("lib.rs");
        fs::write(&file_b, "")?;
        let file_c = other_src.join("component.js");
        fs::write(&file_c, "")?;

        let mut context_a = FileContext::new(file_a, root_path.to_path_buf());
        let mut context_b = FileContext::new(file_b, root_path.to_path_buf());
        let mut context_c = FileContext::new(file_c, root_path.to_path_buf());

        // --- Test Cases ---

        // 1. `**/src` should match files in any `src` directory
        assert!(evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "**/src")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context_b, &PredicateKey::In, "**/src")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut context_c, &PredicateKey::In, "**/src")?
            .is_match());

        // 2. `project_*/src` glob should match relative to the root.
        assert!(evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "project_a/src")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context_b, &PredicateKey::In, "project_*/src")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context_c, &PredicateKey::In, "project_*/src")?
            .is_match());

        // 3. More specific glob `**/project_a/s?c`
        assert!(evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "**/project_a/s?c")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context_b, &PredicateKey::In, "**/project_a/s?c")?
            .is_match());

        // 4. Glob that should not match anything
        assert!(!evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "**/test")?
            .is_match());

        // 5. Glob matching a different directory `**/so*ce`
        assert!(!evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "**/so*ce")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut context_b, &PredicateKey::In, "**/so*ce")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context_c, &PredicateKey::In, "**/so*ce")?
            .is_match());

        Ok(())
    }

    #[test]
    fn test_in_path_evaluator_relative_wildcard() -> Result<()> {
        let evaluator = InPathEvaluator;

        // Create a temporary directory structure
        let root_dir = tempdir()?;
        let root_path = root_dir.path();

        let project_a_src = root_path.join("project_a").join("src");
        fs::create_dir_all(&project_a_src)?;

        let file_a = project_a_src.join("main.rs");
        fs::write(&file_a, "")?;

        // The context's root is the temporary directory we created.
        let mut context_a = FileContext::new(file_a, root_path.to_path_buf());

        // This glob is relative to the context's root.
        // It should match `.../project_a/src`
        assert!(evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "project_a/*")?
            .is_match());

        // This glob should not match.
        assert!(!evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "project_b/*")?
            .is_match());
            
        // This glob should also match.
        assert!(evaluator
            .evaluate(&mut context_a, &PredicateKey::In, "project_a/s?c")?
            .is_match());

        Ok(())
    }

    #[test]
    fn test_in_path_evaluator_advanced_wildcards() -> Result<()> {
        let evaluator = InPathEvaluator;
        let root_dir = tempdir()?;
        let root_path = root_dir.path();

        // Structure:
        //  /src/api/routes.js
        //  /src/db/connect.js
        //  /lib/auth/token.js
        //  /lib/deep/down/utils.js
        let src_api = root_path.join("src").join("api");
        let src_db = root_path.join("src").join("db");
        let lib_auth = root_path.join("lib").join("auth");
        let lib_deep = root_path.join("lib").join("deep").join("down");
        fs::create_dir_all(&src_api)?;
        fs::create_dir_all(&src_db)?;
        fs::create_dir_all(&lib_auth)?;
        fs::create_dir_all(&lib_deep)?;

        let file_in_src_api = src_api.join("routes.js");
        fs::write(&file_in_src_api, "")?;
        let file_in_src_db = src_db.join("connect.js");
        fs::write(&file_in_src_db, "")?;
        let file_in_lib_auth = lib_auth.join("token.js");
        fs::write(&file_in_lib_auth, "")?;
        let file_in_lib_deep = lib_deep.join("utils.js");
        fs::write(&file_in_lib_deep, "")?;

        let mut ctx_api = FileContext::new(file_in_src_api, root_path.to_path_buf());
        let mut ctx_db = FileContext::new(file_in_src_db, root_path.to_path_buf());
        let mut ctx_auth = FileContext::new(file_in_lib_auth, root_path.to_path_buf());
        let mut ctx_deep = FileContext::new(file_in_lib_deep, root_path.to_path_buf());

        // Test single-level wildcard `in:'src/*'`
        assert!(evaluator
            .evaluate(&mut ctx_api, &PredicateKey::In, "src/*")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut ctx_db, &PredicateKey::In, "src/*")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut ctx_auth, &PredicateKey::In, "src/*")?
            .is_match());

        // Test recursive globstar `in:'**/*'`
        assert!(evaluator
            .evaluate(&mut ctx_deep, &PredicateKey::In, "lib/**")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut ctx_deep, &PredicateKey::In, "lib/d*p/**")?
            .is_match());
         assert!(evaluator
            .evaluate(&mut ctx_auth, &PredicateKey::In, "lib/**")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut ctx_api, &PredicateKey::In, "lib/**")?
            .is_match());

        // Test non-matching wildcard
        assert!(!evaluator
            .evaluate(&mut ctx_api, &PredicateKey::In, "dist/*")?
            .is_match());

        Ok(())
    }
}
