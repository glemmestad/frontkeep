-- 0015: standards move from embedded include_str! constants into the DB so they
-- can be edited (admin-only) and versioned like guidance. Keyed by id (matches
-- the /api/standards/{id} route). Rows are always published (standards are
-- normative, no draft queue) but every edit still writes a version. The embedded
-- STANDARDS const stays as the single seed source into an empty table.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
CREATE TABLE IF NOT EXISTS standards (
    id         TEXT PRIMARY KEY,
    title      TEXT NOT NULL,
    summary    TEXT NOT NULL DEFAULT '',
    body       TEXT NOT NULL DEFAULT '',
    author     TEXT NOT NULL DEFAULT '',
    status     TEXT NOT NULL DEFAULT 'published',
    updated_at TEXT NOT NULL
);
