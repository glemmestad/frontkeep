use crate::model::Runtime;

/// An install destination: a runtime's on-disk skills directory and which translation
/// it receives. Cursor reads the Agent Skills standard, so it reuses the Claude Code
/// rendering rather than being a separate translation target.
pub struct Destination {
    pub key: &'static str,
    pub runtime: Runtime,
    /// Display form of the install directory (`~` expands on the client).
    pub dir: &'static str,
}

pub const DESTINATIONS: &[Destination] = &[
    Destination {
        key: "claude-code",
        runtime: Runtime::ClaudeCode,
        dir: "~/.claude/skills",
    },
    Destination {
        key: "codex",
        runtime: Runtime::Codex,
        dir: "~/.codex/skills",
    },
    Destination {
        key: "cursor",
        runtime: Runtime::ClaudeCode,
        dir: "~/.cursor/skills",
    },
];

pub fn destination(key: &str) -> Option<&'static Destination> {
    DESTINATIONS.iter().find(|d| d.key == key)
}

/// A filesystem-safe slug from a skill name — lowercase, non-alphanumeric runs
/// collapsed to single dashes, trimmed. Empty input yields `skill`. Kept in sync
/// with the UI's `skillSlug` so the install directory matches across surfaces.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let s = out.trim_matches('-');
    if s.is_empty() {
        "skill".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_matches_ui_rules() {
        assert_eq!(slug("CSV profiler"), "csv-profiler");
        assert_eq!(slug("  Foo!! Bar  "), "foo-bar");
        assert_eq!(slug("Changelog from commits"), "changelog-from-commits");
        assert_eq!(slug(""), "skill");
        assert_eq!(slug("--- @@@ ---"), "skill");
    }

    #[test]
    fn destinations_map_runtime_and_dir() {
        assert_eq!(destination("cursor").unwrap().runtime, Runtime::ClaudeCode);
        assert_eq!(destination("cursor").unwrap().dir, "~/.cursor/skills");
        assert_eq!(destination("codex").unwrap().runtime, Runtime::Codex);
        assert_eq!(destination("claude-code").unwrap().dir, "~/.claude/skills");
        assert!(destination("vim").is_none());
    }
}
