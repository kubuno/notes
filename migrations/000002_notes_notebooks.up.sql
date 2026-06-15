-- Notebooks hiérarchiques

CREATE TABLE IF NOT EXISTS notebooks (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id    UUID NOT NULL,
    parent_id   UUID REFERENCES notebooks(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    icon        VARCHAR(50) NOT NULL DEFAULT '📁',
    color       VARCHAR(7),
    position    INTEGER NOT NULL DEFAULT 0,
    note_count  INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, parent_id, name)
);

CREATE INDEX IF NOT EXISTS idx_notebooks_owner  ON notebooks(owner_id);
CREATE INDEX IF NOT EXISTS idx_notebooks_parent ON notebooks(parent_id);

CREATE TRIGGER notebooks_updated_at
    BEFORE UPDATE ON notebooks
    FOR EACH ROW EXECUTE FUNCTION set_notes_updated_at();

CREATE OR REPLACE FUNCTION update_notebook_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.notebook_id IS NOT NULL THEN
        UPDATE notebooks SET note_count = note_count + 1 WHERE id = NEW.notebook_id;
    ELSIF TG_OP = 'UPDATE' THEN
        IF OLD.notebook_id IS DISTINCT FROM NEW.notebook_id THEN
            IF OLD.notebook_id IS NOT NULL THEN
                UPDATE notebooks SET note_count = GREATEST(note_count - 1, 0) WHERE id = OLD.notebook_id;
            END IF;
            IF NEW.notebook_id IS NOT NULL THEN
                UPDATE notebooks SET note_count = note_count + 1 WHERE id = NEW.notebook_id;
            END IF;
        END IF;
    ELSIF TG_OP = 'DELETE' AND OLD.notebook_id IS NOT NULL THEN
        UPDATE notebooks SET note_count = GREATEST(note_count - 1, 0) WHERE id = OLD.notebook_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notes_notebook_count
    AFTER INSERT OR UPDATE OF notebook_id OR DELETE ON notes
    FOR EACH ROW EXECUTE FUNCTION update_notebook_count();

ALTER TABLE notes
    ADD CONSTRAINT fk_notes_notebook
    FOREIGN KEY (notebook_id) REFERENCES notebooks(id)
    ON DELETE SET NULL;
