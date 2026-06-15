-- ── Notes : contenu déplacé vers le module files (.kbnot) ─────────────────────
-- Le contenu (markdown + HTML) ne vit plus en base. Reste : une référence
-- file_id, un aperçu tronqué (preview) pour la liste, et le search_vector dérivé
-- (index FTS calculé à l'écriture par l'application — le contenu n'est pas stocké).

-- Le trigger/fonction de search_vector référençaient NEW.content : on les retire,
-- l'application calcule désormais le search_vector directement.
DROP TRIGGER IF EXISTS notes_search_vector ON notes;
DROP FUNCTION IF EXISTS update_notes_search_vector();

ALTER TABLE notes ADD COLUMN IF NOT EXISTS file_id UUID;
ALTER TABLE notes ADD COLUMN IF NOT EXISTS preview TEXT NOT NULL DEFAULT '';

ALTER TABLE notes DROP COLUMN IF EXISTS content;
ALTER TABLE notes DROP COLUMN IF EXISTS content_html;
