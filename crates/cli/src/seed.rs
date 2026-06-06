//! `asgard seed apply` / `bootstrap --write`: take the `bootstrap` tool's output
//! (`{ files: [{path, body}] }`) and write each file to disk under a destination
//! directory. The MCP only returns the bodies and tells the agent to write them;
//! the CLI is what actually creates the repo — the biggest ergonomic win over the
//! agent surface.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wrote,
    Skipped,
    Would,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Wrote => "wrote",
            Action::Skipped => "skipped (exists; use --force)",
            Action::Would => "would write",
        }
    }
}

/// Apply a `bootstrap` result to disk under `dest`. With `write == false` it's a
/// dry run (reports what *would* be written). Existing files are skipped unless
/// `force`.
pub fn apply(
    value: &Value,
    dest: &Path,
    write: bool,
    force: bool,
) -> Result<Vec<(PathBuf, Action)>, CliError> {
    let files = value
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| CliError::Mcp("bootstrap output has no `files` array".into()))?;
    let mut results = Vec::new();
    for f in files {
        let rel = f
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| CliError::Mcp("seed file entry is missing `path`".into()))?;
        let body = f.get("body").and_then(Value::as_str).unwrap_or("");
        let path = dest.join(rel);
        if !write {
            results.push((path, Action::Would));
            continue;
        }
        if path.exists() && !force {
            results.push((path, Action::Skipped));
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::Io(e.to_string()))?;
        }
        std::fs::write(&path, body).map_err(|e| CliError::Io(e.to_string()))?;
        results.push((path, Action::Wrote));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_files_and_respects_dry_run_and_force() {
        let dir = std::env::temp_dir().join(format!("asgard-seed-{}", asgard_storage::new_uid()));
        let plan = json!({
            "files": [
                {"path": "AGENTS.md", "body": "hello"},
                {"path": ".agent/RUST.md", "body": "rust"},
            ]
        });

        // Dry run writes nothing.
        let res = apply(&plan, &dir, false, false).unwrap();
        assert!(res.iter().all(|(_, a)| *a == Action::Would));
        assert!(!dir.join("AGENTS.md").exists());

        // Write creates files (including nested dirs).
        let res = apply(&plan, &dir, true, false).unwrap();
        assert!(res.iter().all(|(_, a)| *a == Action::Wrote));
        assert_eq!(
            std::fs::read_to_string(dir.join("AGENTS.md")).unwrap(),
            "hello"
        );
        assert!(dir.join(".agent/RUST.md").exists());

        // Re-applying without force skips existing files.
        let res = apply(&plan, &dir, true, false).unwrap();
        assert!(res.iter().all(|(_, a)| *a == Action::Skipped));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_files_array_errors() {
        assert!(apply(&json!({"x": 1}), Path::new("."), false, false).is_err());
    }
}
