use crate::predicates::code_aware::profiles::list_language_profiles;
use crate::LangAction;
use anyhow::{anyhow, Result};

pub fn run_lang(action: LangAction) -> Result<()> {
    match action {
        LangAction::List => {
            let profiles = list_language_profiles();
            println!("{:<12} {}", "NAME", "EXTENSIONS");
            println!("──────────────────────────");
            for profile in profiles {
                println!(
                    "{:<12} {}",
                    profile.name,
                    profile.extensions.join(", ")
                );
            }
        }
        LangAction::Describe { language } => {
            let lang_lower = language.to_lowercase();
            let profiles = list_language_profiles();
            let profile = profiles
                .into_iter()
                .find(|p| p.name.to_lowercase() == lang_lower || p.extensions.contains(&lang_lower.as_str()))
                .ok_or_else(|| anyhow!("Language '{}' not supported. Run `rdump lang list` to see available languages.", language))?;

            println!("Predicates for {} ({})", profile.name, profile.extensions.join(", "));

            let metadata_preds = vec!["ext", "name", "path", "size", "modified"];
            let content_preds = vec!["contains", "matches"];

            println!("\nMETADATA");
            println!("  {}", metadata_preds.join(", "));

            println!("\nCONTENT");
            println!("  {}", content_preds.join(", "));

            let semantic_preds: Vec<&str> = profile.queries.keys().map(|k| k.as_ref()).collect();
            if !semantic_preds.is_empty() {
                println!("\nSEMANTIC");
                println!("  {}", semantic_preds.join(", "));
            }
        }
    }
    Ok(())
}
