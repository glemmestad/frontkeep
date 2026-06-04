-- 0006: guidance — governed how-to playbooks, authored by a human (UI) or an
-- agent (MCP `guidance_put`) and read by both surfaces. Distinct from embedded
-- `standards` (normative, shipped in the binary) and the agent-seed (repo
-- bootstrap): guidance is advisory, dynamic, and editable at runtime.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
CREATE TABLE IF NOT EXISTS guidance (
    slug       TEXT PRIMARY KEY,
    title      TEXT NOT NULL,
    summary    TEXT NOT NULL DEFAULT '',
    body       TEXT NOT NULL DEFAULT '',
    tags       TEXT NOT NULL DEFAULT '',
    author     TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL
);
