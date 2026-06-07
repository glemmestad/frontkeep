-- 0027: per-resource retry bookkeeping for async provisioning. attempts counts
-- failed apply/destroy tries. next_retry_at is the materialized backoff deadline
-- (NULL once a row hits its per-service cap, so capped rows drop out of the retry
-- sweep). Keep semicolons out of these comments — the splitter splits on them.
ALTER TABLE provisioned_resources ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE provisioned_resources ADD COLUMN next_retry_at TEXT;
