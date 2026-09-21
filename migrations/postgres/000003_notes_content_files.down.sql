ALTER TABLE notes ADD COLUMN IF NOT EXISTS content TEXT NOT NULL DEFAULT '';
ALTER TABLE notes ADD COLUMN IF NOT EXISTS content_html TEXT;
ALTER TABLE notes DROP COLUMN IF EXISTS preview;
ALTER TABLE notes DROP COLUMN IF EXISTS file_id;

CREATE OR REPLACE FUNCTION update_notes_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('french', unaccent(COALESCE(NEW.title, ''))),    'A') ||
        setweight(to_tsvector('french', unaccent(COALESCE(NEW.content, ''))),  'B') ||
        setweight(to_tsvector('french', unaccent(COALESCE(NEW.transcript, ''))), 'C');
    NEW.word_count := COALESCE(
        array_length(regexp_split_to_array(trim(COALESCE(NEW.content, '')), '\s+'), 1), 0);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notes_search_vector
    BEFORE INSERT OR UPDATE OF title, content, transcript ON notes
    FOR EACH ROW EXECUTE FUNCTION update_notes_search_vector();
