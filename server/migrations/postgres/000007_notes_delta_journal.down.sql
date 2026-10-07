DROP TABLE IF EXISTS change_counter;

-- ── notes ────────────────────────────────────────────────────────────────────
CREATE SEQUENCE IF NOT EXISTS note_change_seq;
ALTER TABLE notes ALTER COLUMN change_seq SET DEFAULT nextval('note_change_seq');
CREATE OR REPLACE FUNCTION notes_bump_note_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('note_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_notes_change_seq BEFORE UPDATE ON notes
    FOR EACH ROW EXECUTE FUNCTION notes_bump_note_seq();

CREATE OR REPLACE FUNCTION notes_note_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO note_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('note_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
CREATE TRIGGER trg_notes_tombstone AFTER DELETE ON notes
    FOR EACH ROW EXECUTE FUNCTION notes_note_tombstone();

CREATE OR REPLACE FUNCTION notes_nl_bump_note() RETURNS trigger AS $$
BEGIN
    UPDATE notes SET change_seq = change_seq WHERE id = COALESCE(NEW.note_id, OLD.note_id);
    RETURN COALESCE(NEW, OLD);
END; $$ LANGUAGE plpgsql;
CREATE TRIGGER trg_note_labels_bump AFTER INSERT OR DELETE ON note_labels
    FOR EACH ROW EXECUTE FUNCTION notes_nl_bump_note();

-- ── notebooks ─────────────────────────────────────────────────────────────────
CREATE SEQUENCE IF NOT EXISTS notebook_change_seq;
ALTER TABLE notebooks ALTER COLUMN change_seq SET DEFAULT nextval('notebook_change_seq');
CREATE OR REPLACE FUNCTION notes_bump_notebook_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('notebook_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_notebooks_change_seq BEFORE UPDATE ON notebooks
    FOR EACH ROW EXECUTE FUNCTION notes_bump_notebook_seq();

CREATE OR REPLACE FUNCTION notes_notebook_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO notebook_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('notebook_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
CREATE TRIGGER trg_notebooks_tombstone AFTER DELETE ON notebooks
    FOR EACH ROW EXECUTE FUNCTION notes_notebook_tombstone();

CREATE OR REPLACE FUNCTION update_notebook_count() RETURNS TRIGGER AS $$
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

-- ── labels ────────────────────────────────────────────────────────────────────
CREATE SEQUENCE IF NOT EXISTS label_change_seq;
ALTER TABLE labels ALTER COLUMN change_seq SET DEFAULT nextval('label_change_seq');
CREATE OR REPLACE FUNCTION notes_bump_label_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('label_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_labels_change_seq BEFORE UPDATE ON labels
    FOR EACH ROW EXECUTE FUNCTION notes_bump_label_seq();

CREATE OR REPLACE FUNCTION notes_label_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO label_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('label_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
CREATE TRIGGER trg_labels_tombstone AFTER DELETE ON labels
    FOR EACH ROW EXECUTE FUNCTION notes_label_tombstone();
