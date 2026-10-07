-- Move full-text search off PostgreSQL's `tsvector` / `to_tsvector` / `ts_rank`
-- and onto `kubuno_db::search`: title, body (the note content) and a voice
-- note's transcript are reduced to Snowball French stems and deaccented IN RUST
-- at write time and stored in plain `TEXT` columns, then a query is put through
-- the same reduction and matched with a portable `LIKE`. Because the stemming
-- happens before any SQL, the stored and searched tokens are byte-for-byte
-- identical on PostgreSQL, MySQL and SQLite. The MySQL and SQLite migrations
-- declare the `*_norm` columns from their CREATE TABLE; here the PostgreSQL
-- table sheds its `tsvector` machinery and gains the columns.
--
-- The `search_vector` trigger/function were already dropped in 000003 (the note
-- body moved to a `.kbnot` file); the column itself lingered. It goes here.
--
-- NOTE: the `unaccent` / `pg_trgm` extensions are NOT dropped — notes never
-- created them (they are provided by the core's own migrations and shared by
-- other modules).
--
-- What is lost: `pg_trgm`'s typo tolerance (a `LIKE '%stem%'` needs the stem to
-- appear as a substring). Stemming still folds inflections and the normalizer
-- folds accents, so inflected and accented queries still match.

-- Retire the tsvector column and its GIN index.
DROP INDEX IF EXISTS idx_notes_search;
ALTER TABLE notes DROP COLUMN IF EXISTS search_vector;

-- One normalized TEXT column per weight class (the portable stand-in for
-- `setweight A/B/C`): title (weight A), body (weight B) and transcript
-- (weight C). Existing rows get an empty string; every save recomputes them in
-- Rust.
ALTER TABLE notes ADD COLUMN IF NOT EXISTS title_norm      TEXT NOT NULL DEFAULT '';
ALTER TABLE notes ADD COLUMN IF NOT EXISTS body_norm       TEXT NOT NULL DEFAULT '';
ALTER TABLE notes ADD COLUMN IF NOT EXISTS transcript_norm TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_notes_title_norm ON notes(title_norm);
CREATE INDEX IF NOT EXISTS idx_notes_body_norm  ON notes(body_norm);

-- Backlink arrays: `UUID[]` has no portable form (MySQL/SQLite have no array
-- type), so `mentions` / `mentioned_by` become JSON arrays of UUID strings —
-- read back through `#[sqlx(json)] Vec<Uuid>`, filtered with the dialect's
-- json-array helpers, written as a whole `Vec<Uuid>` from Rust. `to_jsonb` of a
-- `uuid[]` yields exactly the array of hyphenated strings `JsonVec<Uuid>` decodes.
ALTER TABLE notes ALTER COLUMN mentions     DROP DEFAULT;
ALTER TABLE notes ALTER COLUMN mentions     TYPE JSONB USING to_jsonb(mentions);
ALTER TABLE notes ALTER COLUMN mentions     SET DEFAULT '[]'::jsonb;
ALTER TABLE notes ALTER COLUMN mentioned_by DROP DEFAULT;
ALTER TABLE notes ALTER COLUMN mentioned_by TYPE JSONB USING to_jsonb(mentioned_by);
ALTER TABLE notes ALTER COLUMN mentioned_by SET DEFAULT '[]'::jsonb;
