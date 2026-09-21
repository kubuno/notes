-- SQLite — `notes` is an ATTACHed database file, attached on every pooled
-- connection by kubuno-db, so the qualified names below resolve as they do on
-- the other two engines. This single file declares the FINAL shape the
-- PostgreSQL side reached across its 000001..000007 migrations.
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BLOB, TIMESTAMPTZ -> TEXT (`%F %T%.f`, UTC), as sqlx encodes them.
--   * No DEFAULT on `id`: SQLite has no UUID generator; the process supplies it.
--   * JSONB / UUID[] -> TEXT holding JSON (checklist, mentions, mentioned_by).
--   * Full-text search is the normalized-column form (title_norm / body_norm /
--     transcript_norm, filled in Rust): no tsvector, no unaccent.
--   * The delta layer is the journal (change_counter + per-row change_seq) plus
--     the three tombstone tables; no sequences, no triggers. note_count and
--     updated_at are maintained in Rust.
--   * Foreign-key REFERENCES are unqualified (SQLite assumes the same database);
--     kubuno-db enables `PRAGMA foreign_keys`, so CASCADE deletes fire.

CREATE TABLE notes.notebooks (
    id          BLOB    NOT NULL PRIMARY KEY,
    owner_id    BLOB    NOT NULL,
    parent_id   BLOB    REFERENCES notebooks(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    icon        TEXT    NOT NULL DEFAULT '📁',
    color       TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    note_count  INTEGER NOT NULL DEFAULT 0,
    change_seq  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (owner_id, parent_id, name)
);
CREATE INDEX notes.idx_notebooks_owner      ON notebooks(owner_id);
CREATE INDEX notes.idx_notebooks_parent     ON notebooks(parent_id);
CREATE INDEX notes.idx_notebooks_change_seq ON notebooks(owner_id, change_seq);

CREATE TABLE notes.notes (
    id              BLOB    NOT NULL PRIMARY KEY,
    owner_id        BLOB    NOT NULL,
    notebook_id     BLOB    REFERENCES notebooks(id) ON DELETE SET NULL,
    title           TEXT,
    note_type       TEXT    NOT NULL DEFAULT 'text'
                        CHECK (note_type IN ('text', 'checklist', 'drawing', 'voice')),
    color           TEXT    NOT NULL DEFAULT 'default',
    checklist       TEXT    NOT NULL,
    drawing_path    TEXT,
    audio_path      TEXT,
    transcript      TEXT,
    mentions        TEXT    NOT NULL,
    mentioned_by    TEXT    NOT NULL,
    is_pinned       INTEGER NOT NULL DEFAULT 0,
    is_archived     INTEGER NOT NULL DEFAULT 0,
    is_trashed      INTEGER NOT NULL DEFAULT 0,
    trashed_at      TEXT,
    word_count      INTEGER NOT NULL DEFAULT 0,
    file_id         BLOB,
    preview         TEXT    NOT NULL,
    title_norm      TEXT    NOT NULL,
    body_norm       TEXT    NOT NULL,
    transcript_norm TEXT    NOT NULL,
    change_seq      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX notes.idx_notes_owner      ON notes(owner_id);
CREATE INDEX notes.idx_notes_updated    ON notes(owner_id, updated_at);
CREATE INDEX notes.idx_notes_archived   ON notes(owner_id, is_archived);
CREATE INDEX notes.idx_notes_trashed    ON notes(owner_id, is_trashed);
CREATE INDEX notes.idx_notes_type       ON notes(owner_id, note_type);
CREATE INDEX notes.idx_notes_change_seq ON notes(owner_id, change_seq);
CREATE INDEX notes.idx_notes_title_norm ON notes(title_norm);
CREATE INDEX notes.idx_notes_body_norm  ON notes(body_norm);

CREATE TABLE notes.labels (
    id         BLOB    NOT NULL PRIMARY KEY,
    owner_id   BLOB    NOT NULL,
    name       TEXT    NOT NULL,
    color      TEXT    NOT NULL DEFAULT '#1a73e8',
    position   INTEGER NOT NULL DEFAULT 0,
    change_seq INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (owner_id, name)
);
CREATE INDEX notes.idx_labels_owner      ON labels(owner_id);
CREATE INDEX notes.idx_labels_change_seq ON labels(owner_id, change_seq);

CREATE TABLE notes.note_labels (
    note_id    BLOB NOT NULL REFERENCES notes(id)  ON DELETE CASCADE,
    label_id   BLOB NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (note_id, label_id)
);
CREATE INDEX notes.idx_note_labels_label ON note_labels(label_id);

CREATE TABLE notes.reminders (
    id          BLOB    NOT NULL PRIMARY KEY,
    note_id     BLOB    NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    owner_id    BLOB    NOT NULL,
    fire_at     TEXT    NOT NULL,
    method      TEXT    NOT NULL DEFAULT 'notification'
                    CHECK (method IN ('notification', 'email')),
    recurrence  TEXT    NOT NULL DEFAULT 'once'
                    CHECK (recurrence IN ('once', 'daily', 'weekly')),
    sent_at     TEXT,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX notes.idx_reminders_fire ON reminders(fire_at);
CREATE INDEX notes.idx_reminders_note ON reminders(note_id);

CREATE TABLE notes.shares (
    id               BLOB    NOT NULL PRIMARY KEY,
    note_id          BLOB    NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    created_by       BLOB    NOT NULL,
    token            TEXT    NOT NULL UNIQUE,
    permission       TEXT    NOT NULL DEFAULT 'read',
    password_hash    TEXT,
    expires_at       TEXT,
    view_count       INTEGER NOT NULL DEFAULT 0,
    is_active        INTEGER NOT NULL DEFAULT 1,
    last_accessed_at TEXT,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX notes.idx_shares_note ON shares(note_id);

-- ── Delta journal (portable change layer) ────────────────────────────────────
CREATE TABLE notes.change_counter (
    domain TEXT   NOT NULL PRIMARY KEY,
    n      BIGINT NOT NULL
);

CREATE TABLE notes.note_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX notes.idx_note_tomb_seq ON note_tombstones(owner_id, change_seq);

CREATE TABLE notes.notebook_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX notes.idx_notebook_tomb_seq ON notebook_tombstones(owner_id, change_seq);

CREATE TABLE notes.label_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX notes.idx_label_tomb_seq ON label_tombstones(owner_id, change_seq);
