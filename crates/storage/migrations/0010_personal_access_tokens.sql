-- 0010: personal access tokens (PATs) — a long-lived, user-scoped credential a
-- logged-in user mints for their agent. Unlike a project virtual key (one key =
-- one project), a PAT acts as the user across every project they own or manage,
-- and can register new ones. Stored hashed (sha256), revocable, listable —
-- mirrors the sessions table.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
CREATE TABLE IF NOT EXISTS personal_access_tokens (
    id            TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL,
    name          TEXT NOT NULL DEFAULT '',
    token_hash    TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    expires_at    TEXT,
    revoked_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_pat_token_hash ON personal_access_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_pat_user ON personal_access_tokens(user_id);
