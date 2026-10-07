-- Restore the array columns.
ALTER TABLE notes ALTER COLUMN mentioned_by DROP DEFAULT;
ALTER TABLE notes ALTER COLUMN mentioned_by TYPE UUID[] USING
    ARRAY(SELECT jsonb_array_elements_text(mentioned_by))::UUID[];
ALTER TABLE notes ALTER COLUMN mentioned_by SET DEFAULT '{}';
ALTER TABLE notes ALTER COLUMN mentions DROP DEFAULT;
ALTER TABLE notes ALTER COLUMN mentions TYPE UUID[] USING
    ARRAY(SELECT jsonb_array_elements_text(mentions))::UUID[];
ALTER TABLE notes ALTER COLUMN mentions SET DEFAULT '{}';

-- Drop the normalized columns.
DROP INDEX IF EXISTS idx_notes_body_norm;
DROP INDEX IF EXISTS idx_notes_title_norm;
ALTER TABLE notes DROP COLUMN IF EXISTS transcript_norm;
ALTER TABLE notes DROP COLUMN IF EXISTS body_norm;
ALTER TABLE notes DROP COLUMN IF EXISTS title_norm;

-- Restore the tsvector column and its GIN index (unfilled; a re-save rebuilds it
-- if the old trigger is also restored elsewhere).
ALTER TABLE notes ADD COLUMN IF NOT EXISTS search_vector TSVECTOR;
CREATE INDEX IF NOT EXISTS idx_notes_search ON notes USING GIN(search_vector);
