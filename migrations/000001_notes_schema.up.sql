-- Module Notes — schéma principal

CREATE OR REPLACE FUNCTION set_notes_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS notes (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id        UUID NOT NULL,
    notebook_id     UUID,
    title           VARCHAR(500),
    content         TEXT NOT NULL DEFAULT '',
    content_html    TEXT,
    note_type       VARCHAR(20) NOT NULL DEFAULT 'text'
                        CHECK (note_type IN ('text', 'checklist', 'drawing', 'voice')),
    color           VARCHAR(20) NOT NULL DEFAULT 'default',
    checklist       JSONB NOT NULL DEFAULT '[]',
    drawing_path    TEXT,
    audio_path      TEXT,
    transcript      TEXT,
    mentions        UUID[] NOT NULL DEFAULT '{}',
    mentioned_by    UUID[] NOT NULL DEFAULT '{}',
    is_pinned       BOOLEAN NOT NULL DEFAULT FALSE,
    is_archived     BOOLEAN NOT NULL DEFAULT FALSE,
    is_trashed      BOOLEAN NOT NULL DEFAULT FALSE,
    trashed_at      TIMESTAMPTZ,
    word_count      INTEGER NOT NULL DEFAULT 0,
    search_vector   TSVECTOR,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_notes_owner    ON notes(owner_id);
CREATE INDEX IF NOT EXISTS idx_notes_updated  ON notes(owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_notes_pinned   ON notes(owner_id) WHERE is_pinned = TRUE;
CREATE INDEX IF NOT EXISTS idx_notes_archived ON notes(owner_id, is_archived);
CREATE INDEX IF NOT EXISTS idx_notes_trashed  ON notes(owner_id, is_trashed);
CREATE INDEX IF NOT EXISTS idx_notes_search   ON notes USING GIN(search_vector);
CREATE INDEX IF NOT EXISTS idx_notes_type     ON notes(owner_id, note_type);

CREATE OR REPLACE FUNCTION update_notes_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('french', unaccent(COALESCE(NEW.title, ''))),    'A') ||
        setweight(to_tsvector('french', unaccent(COALESCE(NEW.content, ''))),  'B') ||
        setweight(to_tsvector('french', unaccent(COALESCE(NEW.transcript, ''))), 'C');
    NEW.word_count := COALESCE(
        array_length(regexp_split_to_array(trim(COALESCE(NEW.content, '')), '\s+'), 1),
        0
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER notes_search_vector
    BEFORE INSERT OR UPDATE OF title, content, transcript ON notes
    FOR EACH ROW EXECUTE FUNCTION update_notes_search_vector();

CREATE TRIGGER notes_updated_at
    BEFORE UPDATE ON notes
    FOR EACH ROW EXECUTE FUNCTION set_notes_updated_at();

-- Labels
CREATE TABLE IF NOT EXISTS labels (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id   UUID NOT NULL,
    name       VARCHAR(100) NOT NULL,
    color      VARCHAR(7) NOT NULL DEFAULT '#1a73e8',
    position   INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, name)
);

CREATE INDEX IF NOT EXISTS idx_labels_owner ON labels(owner_id);

CREATE TABLE IF NOT EXISTS note_labels (
    note_id    UUID NOT NULL REFERENCES notes(id)  ON DELETE CASCADE,
    label_id   UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (note_id, label_id)
);

CREATE INDEX IF NOT EXISTS idx_note_labels_label ON note_labels(label_id);

-- Rappels
CREATE TABLE IF NOT EXISTS reminders (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id     UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    owner_id    UUID NOT NULL,
    fire_at     TIMESTAMPTZ NOT NULL,
    method      VARCHAR(20) NOT NULL DEFAULT 'notification'
                    CHECK (method IN ('notification', 'email')),
    recurrence  VARCHAR(10) NOT NULL DEFAULT 'once'
                    CHECK (recurrence IN ('once', 'daily', 'weekly')),
    sent_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reminders_fire ON reminders(fire_at)
    WHERE sent_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_reminders_note ON reminders(note_id);

-- Partages
CREATE TABLE IF NOT EXISTS shares (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    note_id          UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    created_by       UUID NOT NULL,
    token            VARCHAR(64) UNIQUE NOT NULL,
    permission       VARCHAR(20) NOT NULL DEFAULT 'read',
    password_hash    VARCHAR(255),
    expires_at       TIMESTAMPTZ,
    view_count       INTEGER NOT NULL DEFAULT 0,
    is_active        BOOLEAN NOT NULL DEFAULT TRUE,
    last_accessed_at TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shares_token ON shares(token) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_shares_note  ON shares(note_id);
