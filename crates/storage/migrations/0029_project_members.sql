-- 0029: per-project collaborators. A member is an authority over the project's
-- resources (provision, deploy, secrets, keys) without holding the manager slot,
-- which also carries approval authority. Membership is mutable, so unlike
-- owner/manager it is NOT denormalized onto usage_events/cost_rollup -- cost
-- scoping joins through this table. Keep semicolons out of these comments.
CREATE TABLE IF NOT EXISTS project_members (
  project_id TEXT NOT NULL,
  email      TEXT NOT NULL,
  added_by   TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, email)
);
CREATE INDEX IF NOT EXISTS idx_project_members_email ON project_members(email);
