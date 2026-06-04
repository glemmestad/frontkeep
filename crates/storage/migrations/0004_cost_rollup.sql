-- 0004: persisted daily cost rollup, end-of-month forecast, and anomaly flags.
-- The cost actuals computed on demand in Phase 1 are now persisted as a daily
-- time series per (project, service, source). Org dimensions are denormalized at
-- insert time, mirroring usage_events, so every dashboard query is a single-table
-- GROUP BY and historical spend stays attributed to whoever owned the project when
-- it was incurred (never a read-time join to projects_runtime).
-- Portable SQL only (SQLite + Postgres via the Any driver).
-- (Keep semicolons out of comments here, the migration splitter splits on them.)

CREATE TABLE IF NOT EXISTS cost_rollup (
    id             TEXT PRIMARY KEY,
    project_id     TEXT NOT NULL,
    day            TEXT NOT NULL,
    service        TEXT NOT NULL,
    source         TEXT NOT NULL,
    estimated_usd  DOUBLE PRECISION NOT NULL DEFAULT 0,
    actual_usd     DOUBLE PRECISION,
    cumulative_usd DOUBLE PRECISION,
    owner          TEXT NOT NULL DEFAULT '',
    manager        TEXT NOT NULL DEFAULT '',
    cost_group     TEXT NOT NULL DEFAULT '',
    cost_center    TEXT NOT NULL DEFAULT '',
    classification TEXT NOT NULL DEFAULT '',
    created_at     TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_rollup_key ON cost_rollup(project_id, day, service, source);
CREATE INDEX IF NOT EXISTS idx_rollup_day ON cost_rollup(day);
CREATE INDEX IF NOT EXISTS idx_rollup_group ON cost_rollup(cost_group);

CREATE TABLE IF NOT EXISTS cost_forecast (
    project_id  TEXT NOT NULL,
    as_of_day   TEXT NOT NULL,
    method      TEXT NOT NULL DEFAULT 'linreg',
    eom_usd     DOUBLE PRECISION NOT NULL,
    low_usd     DOUBLE PRECISION NOT NULL,
    high_usd    DOUBLE PRECISION NOT NULL,
    r2          DOUBLE PRECISION,
    n_days      INTEGER NOT NULL,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (project_id, as_of_day)
);

CREATE TABLE IF NOT EXISTS cost_anomaly (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL,
    day          TEXT NOT NULL,
    service      TEXT NOT NULL,
    expected_usd DOUBLE PRECISION NOT NULL,
    actual_usd   DOUBLE PRECISION NOT NULL,
    z_score      DOUBLE PRECISION NOT NULL,
    severity     TEXT NOT NULL,
    created_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_anomaly_project ON cost_anomaly(project_id);
