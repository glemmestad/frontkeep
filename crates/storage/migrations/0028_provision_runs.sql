-- 0028: provisioning run-log. One row per connector run (apply/destroy), keyed by
-- the resource it acted on, for operator audit and debugging. output is the full
-- combined stdout+stderr (terraform plan+apply log, exec output, HTTP response),
-- envelope-encrypted (AES-256-GCM, same master key as tf_state) because it can
-- contain provider secrets. Append-only — runs accrete across retries. Keep
-- semicolons out of these comments — the splitter splits on them.
CREATE TABLE IF NOT EXISTS provision_runs (
  id          TEXT PRIMARY KEY,
  resource_id TEXT NOT NULL,
  project_id  TEXT NOT NULL,
  action      TEXT NOT NULL,
  ok          INTEGER NOT NULL,
  ciphertext  TEXT NOT NULL,
  nonce       TEXT NOT NULL,
  started_at  TEXT NOT NULL,
  finished_at TEXT NOT NULL,
  created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_provruns_resource ON provision_runs(resource_id);
