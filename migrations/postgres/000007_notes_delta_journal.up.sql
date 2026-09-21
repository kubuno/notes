-- Move the delta layer off PostgreSQL sequences + triggers and onto the
-- application-driven `kubuno_db::journal` primitive (one shared counter row per
-- domain, seqs taken in Rust at write time, tombstones written in the same
-- transaction). Neither the sequence nor the trigger mechanism has a portable
-- form on MySQL/SQLite, so it is retired here on PostgreSQL too; the tombstone
-- TABLES keep their exact 000004 shape (no data migration), only their triggers
-- go.
--
-- The `notebooks.note_count` maintenance also moves to Rust: its trigger fired a
-- no-op parent bump that MySQL cannot observe, so notes now adjusts the count
-- (and the notebook's change_seq) explicitly on every note write. The trigger is
-- dropped here so PostgreSQL does not double-count.
--
-- The `notes_updated_at` / `notebooks_updated_at` triggers from 000001/000002
-- are deliberately LEFT in place (MySQL uses ON UPDATE, SQLite sets it in Rust);
-- only the change-seq, tombstone and note-count machinery is removed.

-- ── notes: BEFORE UPDATE seq, AFTER DELETE tombstone, note_labels bump ────────
DROP TRIGGER IF EXISTS trg_notes_change_seq ON notes;
DROP TRIGGER IF EXISTS trg_notes_tombstone  ON notes;
DROP TRIGGER IF EXISTS trg_note_labels_bump ON note_labels;
DROP FUNCTION IF EXISTS notes_bump_note_seq();
DROP FUNCTION IF EXISTS notes_note_tombstone();
DROP FUNCTION IF EXISTS notes_nl_bump_note();

-- ── notebooks: BEFORE UPDATE seq, AFTER DELETE tombstone, note_count ──────────
DROP TRIGGER IF EXISTS trg_notebooks_change_seq ON notebooks;
DROP TRIGGER IF EXISTS trg_notebooks_tombstone  ON notebooks;
DROP TRIGGER IF EXISTS notes_notebook_count     ON notes;
DROP FUNCTION IF EXISTS notes_bump_notebook_seq();
DROP FUNCTION IF EXISTS notes_notebook_tombstone();
DROP FUNCTION IF EXISTS update_notebook_count();

-- ── labels: BEFORE UPDATE seq, AFTER DELETE tombstone ────────────────────────
DROP TRIGGER IF EXISTS trg_labels_change_seq ON labels;
DROP TRIGGER IF EXISTS trg_labels_tombstone  ON labels;
DROP FUNCTION IF EXISTS notes_bump_label_seq();
DROP FUNCTION IF EXISTS notes_label_tombstone();

-- The DEFAULTs reference the sequences, so they must go before the sequences do.
ALTER TABLE notes     ALTER COLUMN change_seq SET DEFAULT 0;
ALTER TABLE notebooks ALTER COLUMN change_seq SET DEFAULT 0;
ALTER TABLE labels    ALTER COLUMN change_seq SET DEFAULT 0;
DROP SEQUENCE IF EXISTS note_change_seq;
DROP SEQUENCE IF EXISTS notebook_change_seq;
DROP SEQUENCE IF EXISTS label_change_seq;

-- ── The journal's shared counter, seeded to continue the existing sequences ───
CREATE TABLE IF NOT EXISTS change_counter (
    domain VARCHAR(190) NOT NULL PRIMARY KEY,
    n      BIGINT       NOT NULL
);

-- Seed each domain to the current max so `next_seq` (n := n + 1) never hands out
-- a value an existing row already holds.
INSERT INTO change_counter (domain, n)
    SELECT 'notes', COALESCE(MAX(change_seq), 0) FROM notes
    ON CONFLICT (domain) DO NOTHING;
INSERT INTO change_counter (domain, n)
    SELECT 'notebooks', COALESCE(MAX(change_seq), 0) FROM notebooks
    ON CONFLICT (domain) DO NOTHING;
INSERT INTO change_counter (domain, n)
    SELECT 'labels', COALESCE(MAX(change_seq), 0) FROM labels
    ON CONFLICT (domain) DO NOTHING;

-- The tombstone tables (note_/notebook_/label_tombstones) keep their 000004
-- shape unchanged; only their triggers were dropped above.
