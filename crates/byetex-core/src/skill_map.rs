//! Default warning-category → repair-skill mapping. Used to fill a warning's
//! `suggested_skill` when an emit site didn't set one explicitly, so every
//! warning points an agent at the guide that explains the fix.

use crate::warnings::Category;

/// The skill name that best explains how to act on a warning of this category.
/// Returns `None` only for categories with no actionable guide.
pub fn default_skill_for(cat: &Category) -> Option<&'static str> {
    match cat {
        Category::UnsupportedEnvironment { .. } => Some("byetex-unsupported-environment"),
        Category::Tikz => Some("byetex-tikz-to-typst"),
        Category::CustomMacro { .. } => Some("byetex-custom-macros"),
        Category::ParseError { .. } => Some("byetex-parse-error"),
        Category::AmbiguousMath { .. } => Some("byetex-using-warnings-json"),
        Category::UnsupportedCommand { .. } => Some("byetex-using-warnings-json"),
        Category::NeedsManualReview { .. } => Some("byetex-using-warnings-json"),
        Category::UnknownPackage { .. } => Some("byetex-using-warnings-json"),
        Category::DropOnly { .. } => None,
    }
}
