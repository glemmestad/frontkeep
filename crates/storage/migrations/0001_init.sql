-- Core schema. Portable across SQLite and Postgres: TEXT for ids/json/timestamps
-- (RFC3339), INTEGER for booleans (0/1), DOUBLE PRECISION for money/scores.
-- All ids are generated app-side (UUID/ULID) to avoid AUTOINCREMENT/SERIAL drift.

CREATE TABLE IF NOT EXISTS entities (
    uid           TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    namespace     TEXT NOT NULL,
    name          TEXT NOT NULL,
    title         TEXT,
    description   TEXT,
    spec          TEXT NOT NULL DEFAULT '{}',
    metadata      TEXT NOT NULL DEFAULT '{}',
    lifecycle     TEXT NOT NULL DEFAULT 'active',
    origin_repo   TEXT,
    origin_path   TEXT,
    origin_commit TEXT,
    source_id     TEXT,
    content_hash  TEXT NOT NULL DEFAULT '',
    seen_at       TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    deleted_at    TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_entities_knn ON entities(kind, namespace, name);
CREATE INDEX IF NOT EXISTS idx_entities_source ON entities(source_id);
CREATE INDEX IF NOT EXISTS idx_entities_kind ON entities(kind);

CREATE TABLE IF NOT EXISTS relations (
    from_uid  TEXT NOT NULL,
    rel_type  TEXT NOT NULL,
    to_ref    TEXT NOT NULL,
    to_uid    TEXT
);
CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(from_uid);
CREATE INDEX IF NOT EXISTS idx_relations_toref ON relations(to_ref);

CREATE TABLE IF NOT EXISTS audit_log (
    id          TEXT PRIMARY KEY,
    ts          TEXT NOT NULL,
    actor       TEXT NOT NULL,
    action      TEXT NOT NULL,
    entity_ref  TEXT,
    trace_id    TEXT,
    outcome     TEXT NOT NULL,
    reason      TEXT,
    data        TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_log(ts);
CREATE INDEX IF NOT EXISTS idx_audit_trace ON audit_log(trace_id);
CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_log(entity_ref);

CREATE TABLE IF NOT EXISTS virtual_keys (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL,
    key_hash     TEXT NOT NULL,
    key_prefix   TEXT NOT NULL,
    name         TEXT,
    active       INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL,
    revoked_at   TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_vkeys_hash ON virtual_keys(key_hash);
CREATE INDEX IF NOT EXISTS idx_vkeys_project ON virtual_keys(project_id);

CREATE TABLE IF NOT EXISTS projects_runtime (
    project_id    TEXT PRIMARY KEY,
    budget_usd    DOUBLE PRECISION NOT NULL DEFAULT 0,
    spent_usd     DOUBLE PRECISION NOT NULL DEFAULT 0,
    killed        INTEGER NOT NULL DEFAULT 0,
    data_class    TEXT NOT NULL DEFAULT 'public',
    updated_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS usage_events (
    id                TEXT PRIMARY KEY,
    ts                TEXT NOT NULL,
    project_id        TEXT NOT NULL,
    trace_id          TEXT,
    model             TEXT NOT NULL,
    provider          TEXT NOT NULL,
    prompt_tokens     INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd          DOUBLE PRECISION NOT NULL DEFAULT 0,
    latency_ms        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_usage_project ON usage_events(project_id);
CREATE INDEX IF NOT EXISTS idx_usage_ts ON usage_events(ts);

CREATE TABLE IF NOT EXISTS workflow_requests (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL,
    requester     TEXT NOT NULL,
    subject       TEXT NOT NULL,
    state         TEXT NOT NULL,
    approver      TEXT,
    payload       TEXT NOT NULL DEFAULT '{}',
    reason        TEXT,
    sla_due_at    TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_wf_state ON workflow_requests(state);
CREATE INDEX IF NOT EXISTS idx_wf_requester ON workflow_requests(requester);

CREATE TABLE IF NOT EXISTS eval_runs (
    id            TEXT PRIMARY KEY,
    eval_ref      TEXT NOT NULL,
    ts            TEXT NOT NULL,
    pass_rate     DOUBLE PRECISION NOT NULL DEFAULT 0,
    avg_score     DOUBLE PRECISION NOT NULL DEFAULT 0,
    verdict       TEXT NOT NULL,
    detail        TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_evalruns_ref ON eval_runs(eval_ref);

CREATE TABLE IF NOT EXISTS users (
    id            TEXT PRIMARY KEY,
    username      TEXT NOT NULL,
    email         TEXT,
    display_name  TEXT,
    password_hash TEXT,
    provider      TEXT NOT NULL DEFAULT 'local',
    is_admin      INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_username ON users(username);

CREATE TABLE IF NOT EXISTS sessions (
    token_hash    TEXT PRIMARY KEY,
    user_id       TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    expires_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS id_counters (
    scope         TEXT PRIMARY KEY,
    value         INTEGER NOT NULL DEFAULT 0
);
