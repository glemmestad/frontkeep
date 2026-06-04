-- 0013: recipe moderation, mirroring guidance (0008). Anyone (a person or an
-- agent) may submit a recipe, but it is a draft until an admin approves it.
-- Readers see only published recipes. Existing rows were the trusted starter
-- set, so mark them published.
-- (Keep semicolons out of comments here, the migration splitter splits on them.)
ALTER TABLE recipes ADD COLUMN status TEXT NOT NULL DEFAULT 'pending';
UPDATE recipes SET status = 'published';
