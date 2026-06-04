-- 0002: project registration + cost attribution dimensions.
-- projects_runtime becomes the system of record for who owns a project and which
-- cost-center it rolls up to. usage_events denormalizes those dims at insert time
-- so every cost query is a single-table GROUP BY and historical spend stays
-- attributed to whoever owned the project when the cost was incurred.
-- ALTER ADD COLUMN with constant defaults is portable across SQLite and Postgres.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)

ALTER TABLE projects_runtime ADD COLUMN owner TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN manager TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN cost_group TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN cost_center TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN classification TEXT NOT NULL DEFAULT 'poc';
ALTER TABLE projects_runtime ADD COLUMN lifecycle TEXT NOT NULL DEFAULT 'active';
ALTER TABLE projects_runtime ADD COLUMN registered INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects_runtime ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN created_at TEXT NOT NULL DEFAULT '';

ALTER TABLE usage_events ADD COLUMN owner TEXT NOT NULL DEFAULT '';
ALTER TABLE usage_events ADD COLUMN manager TEXT NOT NULL DEFAULT '';
ALTER TABLE usage_events ADD COLUMN cost_group TEXT NOT NULL DEFAULT '';
ALTER TABLE usage_events ADD COLUMN cost_center TEXT NOT NULL DEFAULT '';
ALTER TABLE usage_events ADD COLUMN classification TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_usage_group ON usage_events(cost_group);
CREATE INDEX IF NOT EXISTS idx_usage_owner ON usage_events(owner);
CREATE INDEX IF NOT EXISTS idx_usage_manager ON usage_events(manager);

-- Resources provisioned through the orchestrator. project_id is the join key that
-- ties infra spend into the same per-project rollup as model spend. est_monthly_usd
-- lets the dry-run/stub backend make the cost loop visible before a live billing feed.
CREATE TABLE IF NOT EXISTS provisioned_resources (
    id              TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL,
    rtype           TEXT NOT NULL,
    name            TEXT NOT NULL,
    spec            TEXT NOT NULL DEFAULT '{}',
    outputs         TEXT NOT NULL DEFAULT '{}',
    tags            TEXT NOT NULL DEFAULT '{}',
    est_monthly_usd DOUBLE PRECISION NOT NULL DEFAULT 0,
    state           TEXT NOT NULL DEFAULT 'planned',
    backend         TEXT NOT NULL DEFAULT 'stub',
    dry_run         INTEGER NOT NULL DEFAULT 1,
    request_id      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_provres_project ON provisioned_resources(project_id);
