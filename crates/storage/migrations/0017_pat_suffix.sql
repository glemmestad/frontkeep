-- 0017: store a short, non-secret suffix (last 8 chars) of each PAT so the token
-- list can show which token is which without ever revealing the full value. The
-- full token is shown once at creation and only its sha256 hash is persisted.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
ALTER TABLE personal_access_tokens ADD COLUMN token_suffix TEXT NOT NULL DEFAULT '';
