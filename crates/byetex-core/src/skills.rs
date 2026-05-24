//! Compile-time-embedded skill catalogue.
//!
//! The `build.rs` script enumerates `<workspace>/skills/*.md`, parses the
//! YAML-ish frontmatter, and generates `skills_generated.rs` (in OUT_DIR)
//! containing a `pub static SKILLS: &[Skill]` array. We include that file
//! here and expose a small public API.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Skill {
    pub name: &'static str,
    pub description: &'static str,
    /// Full markdown body including the frontmatter block.
    pub body: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/skills_generated.rs"));

/// All skills bundled with this binary.
pub fn list_skills() -> &'static [Skill] {
    SKILLS
}

/// Look up a skill by its `name` field (matches the frontmatter key).
pub fn read_skill(name: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.name == name)
}
