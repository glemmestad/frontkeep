-- 0014: guidance gains a category facet (best-practice, guide, reference) so the
-- Guidance tab can mirror the source information architecture. Existing rows
-- default to the neutral 'guide'.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
ALTER TABLE guidance ADD COLUMN category TEXT NOT NULL DEFAULT 'guide';
