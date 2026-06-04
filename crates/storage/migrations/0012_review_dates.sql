-- 0012: review-date / expiry engine fields on projects_runtime (WS3).
-- Expiry is a flag, not a lifecycle state: review_state goes ok -> expired but
-- the project's lifecycle stays active, so expiry blocks nothing — it flags and
-- audits for visibility and the portfolio metric. review_extensions counts the
-- automatic extensions granted (the policy allows a bounded number before a
-- human must decide). stack_exception_renewal_date makes exceptions expire so
-- an exception without a renewal date surfaces as unmanaged policy drift.
-- ALTER ADD COLUMN with a constant default is portable across SQLite and Postgres.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)

ALTER TABLE projects_runtime ADD COLUMN review_date TEXT NOT NULL DEFAULT '';
ALTER TABLE projects_runtime ADD COLUMN review_state TEXT NOT NULL DEFAULT 'ok';
ALTER TABLE projects_runtime ADD COLUMN review_extensions INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects_runtime ADD COLUMN stack_exception_renewal_date TEXT NOT NULL DEFAULT '';
