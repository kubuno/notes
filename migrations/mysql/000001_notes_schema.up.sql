-- MySQL / MariaDB — the `notes` database is created by kubuno-db's schema setup
-- before the migrator runs, so there is no CREATE DATABASE here. This single
-- file declares the FINAL shape the PostgreSQL side reached across its
-- 000001..000007 migrations (content moved to `.kbnot` files, delta journal,
-- normalized search columns).
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BINARY(16): what sqlx encodes a `uuid::Uuid` as on MySQL.
--   * No DEFAULT on `id`: MySQL has no gen_random_uuid() and no RETURNING, so
--     the process supplies every primary key.
--   * TIMESTAMPTZ -> DATETIME(6); every value written is UTC (the pool pins
--     `time_zone = '+00:00'`). updated_at uses ON UPDATE CURRENT_TIMESTAMP(6).
--   * JSONB / UUID[] -> JSON (checklist, mentions, mentioned_by). The two array
--     columns carry JSON arrays of UUID strings; the app always binds them.
--   * Full-text search is the normalized-column form (title_norm / body_norm /
--     transcript_norm, filled in Rust): no tsvector, no GIN, no unaccent.
--   * The delta layer is the journal (change_counter + per-row change_seq) plus
--     the three tombstone tables; no sequences, no triggers. note_count is
--     maintained in Rust.
--   * Partial indexes (WHERE ...) become plain indexes (MySQL has none).
--   * utf8mb4_bin so a UNIQUE key stays case- and accent-sensitive.

CREATE TABLE notebooks (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id    BINARY(16)   NOT NULL,
    parent_id   BINARY(16)   NULL,
    name        VARCHAR(255) NOT NULL,
    icon        VARCHAR(50)  NOT NULL DEFAULT '📁',
    color       VARCHAR(7)   NULL,
    position    INT          NOT NULL DEFAULT 0,
    note_count  INT          NOT NULL DEFAULT 0,
    change_seq  BIGINT       NOT NULL DEFAULT 0,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                             ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE (owner_id, parent_id, name),
    FOREIGN KEY (parent_id) REFERENCES notebooks(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_notebooks_owner       ON notebooks(owner_id);
CREATE INDEX idx_notebooks_parent      ON notebooks(parent_id);
CREATE INDEX idx_notebooks_change_seq  ON notebooks(owner_id, change_seq);

CREATE TABLE notes (
    id              BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id        BINARY(16)   NOT NULL,
    notebook_id     BINARY(16)   NULL,
    title           VARCHAR(500) NULL,
    note_type       VARCHAR(20)  NOT NULL DEFAULT 'text'
                        CHECK (note_type IN ('text', 'checklist', 'drawing', 'voice')),
    color           VARCHAR(20)  NOT NULL DEFAULT 'default',
    checklist       JSON         NOT NULL,
    drawing_path    TEXT         NULL,
    audio_path      TEXT         NULL,
    transcript      TEXT         NULL,
    mentions        JSON         NOT NULL,
    mentioned_by    JSON         NOT NULL,
    is_pinned       BOOLEAN      NOT NULL DEFAULT FALSE,
    is_archived     BOOLEAN      NOT NULL DEFAULT FALSE,
    is_trashed      BOOLEAN      NOT NULL DEFAULT FALSE,
    trashed_at      DATETIME(6)  NULL,
    word_count      INT          NOT NULL DEFAULT 0,
    file_id         BINARY(16)   NULL,
    preview         TEXT         NOT NULL,
    title_norm      TEXT         NOT NULL,
    body_norm       TEXT         NOT NULL,
    transcript_norm TEXT         NOT NULL,
    change_seq      BIGINT       NOT NULL DEFAULT 0,
    created_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at      DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                 ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (notebook_id) REFERENCES notebooks(id) ON DELETE SET NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_notes_owner       ON notes(owner_id);
CREATE INDEX idx_notes_updated     ON notes(owner_id, updated_at);
CREATE INDEX idx_notes_archived    ON notes(owner_id, is_archived);
CREATE INDEX idx_notes_trashed     ON notes(owner_id, is_trashed);
CREATE INDEX idx_notes_type        ON notes(owner_id, note_type);
CREATE INDEX idx_notes_change_seq  ON notes(owner_id, change_seq);
CREATE INDEX idx_notes_title_norm  ON notes(title_norm(191));
CREATE INDEX idx_notes_body_norm   ON notes(body_norm(191));

CREATE TABLE labels (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)   NOT NULL,
    name       VARCHAR(100) NOT NULL,
    color      VARCHAR(7)   NOT NULL DEFAULT '#1a73e8',
    position   INT          NOT NULL DEFAULT 0,
    change_seq BIGINT       NOT NULL DEFAULT 0,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (owner_id, name)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_labels_owner       ON labels(owner_id);
CREATE INDEX idx_labels_change_seq  ON labels(owner_id, change_seq);

CREATE TABLE note_labels (
    note_id    BINARY(16)  NOT NULL,
    label_id   BINARY(16)  NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (note_id, label_id),
    FOREIGN KEY (note_id)  REFERENCES notes(id)  ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_note_labels_label ON note_labels(label_id);

CREATE TABLE reminders (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    note_id     BINARY(16)  NOT NULL,
    owner_id    BINARY(16)  NOT NULL,
    fire_at     DATETIME(6) NOT NULL,
    method      VARCHAR(20) NOT NULL DEFAULT 'notification'
                    CHECK (method IN ('notification', 'email')),
    recurrence  VARCHAR(10) NOT NULL DEFAULT 'once'
                    CHECK (recurrence IN ('once', 'daily', 'weekly')),
    sent_at     DATETIME(6) NULL,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_reminders_fire ON reminders(fire_at);
CREATE INDEX idx_reminders_note ON reminders(note_id);

CREATE TABLE shares (
    id               BINARY(16)  NOT NULL PRIMARY KEY,
    note_id          BINARY(16)  NOT NULL,
    created_by       BINARY(16)  NOT NULL,
    token            VARCHAR(64) NOT NULL,
    permission       VARCHAR(20) NOT NULL DEFAULT 'read',
    password_hash    VARCHAR(255) NULL,
    expires_at       DATETIME(6) NULL,
    view_count       INT         NOT NULL DEFAULT 0,
    is_active        BOOLEAN     NOT NULL DEFAULT TRUE,
    last_accessed_at DATETIME(6) NULL,
    created_at       DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (token),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_shares_note ON shares(note_id);

-- ── Delta journal (portable change layer) ────────────────────────────────────
CREATE TABLE change_counter (
    domain VARCHAR(190) NOT NULL PRIMARY KEY,
    n      BIGINT       NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE note_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_note_tomb_seq ON note_tombstones(owner_id, change_seq);

CREATE TABLE notebook_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_notebook_tomb_seq ON notebook_tombstones(owner_id, change_seq);

CREATE TABLE label_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_label_tomb_seq ON label_tombstones(owner_id, change_seq);
