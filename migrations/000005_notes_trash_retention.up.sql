-- Trash retention: the cleaner sweeps notes trashed longer ago than the
-- instance setting `notes.trash_retention_days` allows.
--
-- The existing index is (owner_id, is_trashed) — it answers "this user's bin",
-- not "everything trashed before that date", which is what the sweep asks every
-- hour across all accounts. A partial index on the trashed rows only keeps that
-- scan proportional to the bin instead of the whole table.

CREATE INDEX IF NOT EXISTS idx_notes_trashed_at
    ON notes(trashed_at)
    WHERE is_trashed = TRUE;
