-- 0016: append-only version history shared by guidance, recipes, and standards.
-- The snapshot is the whole doc serialized as JSON rather than typed columns: the
-- three doc types diverge (a recipe carries a spec), so a JSON snapshot keeps this
-- table stable and makes history a passthrough plus UI-side diff. action is one of
-- created, updated, approved.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
CREATE TABLE IF NOT EXISTS knowledge_versions (
    id         TEXT PRIMARY KEY,
    doc_type   TEXT NOT NULL,
    slug       TEXT NOT NULL,
    version    INTEGER NOT NULL,
    action     TEXT NOT NULL,
    author     TEXT NOT NULL DEFAULT '',
    snapshot   TEXT NOT NULL DEFAULT '{}',
    changed_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_knowledge_versions_key ON knowledge_versions (doc_type, slug, version);
