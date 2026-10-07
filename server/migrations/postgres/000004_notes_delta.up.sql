-- Delta primitives for the notes local-first pull sync (notes, notebooks, labels)
-- mirror of office 000033: one monotonic change_seq per entity + tombstones,
-- maintained by triggers. note_labels changes bump the parent NOTE (the sync unit
-- carries its label assignments inline).

-- ── Notes ───────────────────────────────────────────────────────────────────
CREATE SEQUENCE IF NOT EXISTS note_change_seq;
ALTER TABLE notes ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('note_change_seq');
CREATE INDEX IF NOT EXISTS idx_notes_change_seq ON notes(owner_id, change_seq);

CREATE OR REPLACE FUNCTION notes_bump_note_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('note_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notes_change_seq ON notes;
CREATE TRIGGER trg_notes_change_seq BEFORE UPDATE ON notes
    FOR EACH ROW EXECUTE FUNCTION notes_bump_note_seq();

CREATE TABLE IF NOT EXISTS note_tombstones (
    id         UUID        PRIMARY KEY,
    owner_id   UUID        NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_note_tomb_seq ON note_tombstones(owner_id, change_seq);

CREATE OR REPLACE FUNCTION notes_note_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO note_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('note_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notes_tombstone ON notes;
CREATE TRIGGER trg_notes_tombstone AFTER DELETE ON notes
    FOR EACH ROW EXECUTE FUNCTION notes_note_tombstone();

-- Label assignments count as a note change (no-op update fires the seq trigger).
CREATE OR REPLACE FUNCTION notes_nl_bump_note() RETURNS trigger AS $$
BEGIN
    UPDATE notes SET change_seq = change_seq
     WHERE id = COALESCE(NEW.note_id, OLD.note_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_note_labels_bump ON note_labels;
CREATE TRIGGER trg_note_labels_bump AFTER INSERT OR DELETE ON note_labels
    FOR EACH ROW EXECUTE FUNCTION notes_nl_bump_note();

-- ── Notebooks ───────────────────────────────────────────────────────────────
CREATE SEQUENCE IF NOT EXISTS notebook_change_seq;
ALTER TABLE notebooks ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('notebook_change_seq');
CREATE INDEX IF NOT EXISTS idx_notebooks_change_seq ON notebooks(owner_id, change_seq);

CREATE OR REPLACE FUNCTION notes_bump_notebook_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('notebook_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notebooks_change_seq ON notebooks;
CREATE TRIGGER trg_notebooks_change_seq BEFORE UPDATE ON notebooks
    FOR EACH ROW EXECUTE FUNCTION notes_bump_notebook_seq();

CREATE TABLE IF NOT EXISTS notebook_tombstones (
    id         UUID        PRIMARY KEY,
    owner_id   UUID        NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_notebook_tomb_seq ON notebook_tombstones(owner_id, change_seq);

CREATE OR REPLACE FUNCTION notes_notebook_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO notebook_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('notebook_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notebooks_tombstone ON notebooks;
CREATE TRIGGER trg_notebooks_tombstone AFTER DELETE ON notebooks
    FOR EACH ROW EXECUTE FUNCTION notes_notebook_tombstone();

-- ── Labels ──────────────────────────────────────────────────────────────────
CREATE SEQUENCE IF NOT EXISTS label_change_seq;
ALTER TABLE labels ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('label_change_seq');
CREATE INDEX IF NOT EXISTS idx_labels_change_seq ON labels(owner_id, change_seq);

CREATE OR REPLACE FUNCTION notes_bump_label_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('label_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_labels_change_seq ON labels;
CREATE TRIGGER trg_labels_change_seq BEFORE UPDATE ON labels
    FOR EACH ROW EXECUTE FUNCTION notes_bump_label_seq();

CREATE TABLE IF NOT EXISTS label_tombstones (
    id         UUID        PRIMARY KEY,
    owner_id   UUID        NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_label_tomb_seq ON label_tombstones(owner_id, change_seq);

CREATE OR REPLACE FUNCTION notes_label_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO label_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('label_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_labels_tombstone ON labels;
CREATE TRIGGER trg_labels_tombstone AFTER DELETE ON labels
    FOR EACH ROW EXECUTE FUNCTION notes_label_tombstone();
