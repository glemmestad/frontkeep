-- 0008: guidance moderation. Anyone (a person or an agent) may submit guidance,
-- but it is a draft until an admin approves it. Readers see only published docs.
-- Existing rows were the trusted starter set, so mark them published.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
ALTER TABLE guidance ADD COLUMN status TEXT NOT NULL DEFAULT 'pending';
UPDATE guidance SET status = 'published';
