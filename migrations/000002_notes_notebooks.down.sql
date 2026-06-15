ALTER TABLE notes DROP CONSTRAINT IF EXISTS fk_notes_notebook;
DROP TABLE IF EXISTS notebooks;
DROP FUNCTION IF EXISTS update_notebook_count;
