-- 0003: secret store. Provisioning that yields credentials writes the value
-- here (envelope-encrypted by the builtin store) and records only a reference
-- in provisioned_resources. A name is versioned in place: rotation inserts a new
-- row with the next version for (project_id, name) and a stable path, so a ref
-- recorded before a rotation keeps resolving to the latest version.
-- (Keep semicolons out of comments, the migration splitter splits on them.)

CREATE TABLE IF NOT EXISTS secrets (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    name        TEXT NOT NULL,
    version     INTEGER NOT NULL DEFAULT 1,
    ciphertext  TEXT NOT NULL,
    nonce       TEXT NOT NULL,
    meta        TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL,
    rotated_at  TEXT
);
CREATE INDEX IF NOT EXISTS idx_secrets_project ON secrets(project_id);
CREATE INDEX IF NOT EXISTS idx_secrets_name ON secrets(project_id, name);
