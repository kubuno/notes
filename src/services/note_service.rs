use anyhow::{Context, Result};
use uuid::Uuid;

use crate::models::{CreateNoteDto, ListNotesQuery, Note, UpdateNoteDto};
use crate::services::{content_files, markdown_service};
use crate::state::AppState;

/// Peuple `content` (depuis l'aperçu) pour l'affichage en liste — pas de lecture
/// fichier (perf). Le détail (`get_note`) lit le contenu complet du fichier.
fn fill_preview(mut notes: Vec<Note>) -> Vec<Note> {
    for n in &mut notes {
        n.content = n.preview.clone();
    }
    notes
}

pub async fn list_notes(state: &AppState, owner_id: Uuid, q: ListNotesQuery) -> Result<Vec<Note>> {
    let db = &state.db;
    let limit  = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);
    let trashed = q.trashed.unwrap_or(false);

    // Full-text search (index search_vector dérivé)
    if let Some(ref search) = q.search {
        let notes = sqlx::query_as::<_, Note>(
            r#"SELECT * FROM notes
               WHERE owner_id = $1
                 AND is_trashed = $2
                 AND search_vector @@ plainto_tsquery('french', $3)
               ORDER BY ts_rank(search_vector, plainto_tsquery('french', $3)) DESC
               LIMIT $4 OFFSET $5"#,
        )
        .bind(owner_id)
        .bind(trashed)
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
        .context("list_notes search")?;
        return Ok(fill_preview(notes));
    }

    let notes = sqlx::query_as::<_, Note>(
        r#"SELECT * FROM notes
           WHERE owner_id = $1
             AND is_trashed = $6
             AND ($2::uuid IS NULL OR notebook_id = $2)
             AND ($3::text IS NULL OR note_type = $3)
             AND ($4::boolean IS NULL OR is_pinned = $4)
             AND ($5::boolean IS NULL OR is_archived = $5)
             AND ($7::uuid IS NULL OR id IN (
                 SELECT note_id FROM note_labels WHERE label_id = $7
             ))
           ORDER BY is_pinned DESC, updated_at DESC
           LIMIT $8 OFFSET $9"#,
    )
    .bind(owner_id)
    .bind(q.notebook_id)
    .bind(q.note_type.as_deref())
    .bind(q.pinned)
    .bind(q.archived)
    .bind(trashed)
    .bind(q.label_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .context("list_notes")?;

    Ok(fill_preview(notes))
}

/// Récupère la note avec son contenu complet (lu depuis le fichier .kbnot).
pub async fn get_note(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<Option<Note>> {
    let mut note = match sqlx::query_as::<_, Note>(
        "SELECT * FROM notes WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await
    .context("get_note")?
    {
        Some(n) => n,
        None => return Ok(None),
    };

    if let Some(fid) = note.file_id {
        if let Ok((content, html)) = content_files::read_note(state, owner_id, fid).await {
            note.content = content;
            note.content_html = Some(html);
        }
    }
    Ok(Some(note))
}

pub async fn create_note(state: &AppState, owner_id: Uuid, dto: CreateNoteDto) -> Result<Note> {
    let content      = dto.content.unwrap_or_default();
    let content_html = markdown_service::render(&content);
    let note_type    = dto.note_type.as_deref().unwrap_or("text");
    let color        = dto.color.as_deref().unwrap_or("default");
    let checklist    = dto.checklist.unwrap_or(serde_json::json!([]));
    let title        = dto.title.as_deref();

    // Contenu → fichier .kbnot.
    let file_id    = content_files::create_note_file(state, owner_id, title, &content, &content_html)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    let preview    = content_files::make_preview(&content);
    let word_count = content.split_whitespace().count() as i32;

    let mut note = sqlx::query_as::<_, Note>(
        r#"INSERT INTO notes
           (owner_id, notebook_id, title, file_id, preview, note_type, color, checklist, is_pinned, word_count, search_vector)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
               setweight(to_tsvector('french', unaccent(COALESCE($3, ''))), 'A') ||
               setweight(to_tsvector('french', unaccent($11)),               'B'))
           RETURNING *"#,
    )
    .bind(owner_id)
    .bind(dto.notebook_id)
    .bind(title)
    .bind(file_id)
    .bind(&preview)
    .bind(note_type)
    .bind(color)
    .bind(&checklist)
    .bind(dto.is_pinned.unwrap_or(false))
    .bind(word_count)
    .bind(&content)
    .fetch_one(&state.db)
    .await
    .context("create_note")?;

    note.content = content;
    note.content_html = Some(content_html);
    Ok(note)
}

pub async fn update_note(
    state: &AppState,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateNoteDto,
) -> Result<Option<Note>> {
    // Note existante (métadonnée : file_id, title, transcript).
    let existing = match sqlx::query_as::<_, Note>(
        "SELECT * FROM notes WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await
    .context("update_note fetch")?
    {
        Some(n) => n,
        None => return Ok(None),
    };

    // Contenu : nouveau si fourni, sinon lecture du fichier existant.
    let content: String = match dto.content.clone() {
        Some(c) => c,
        None => match existing.file_id {
            Some(fid) => content_files::read_note(state, owner_id, fid).await.map(|(c, _)| c).unwrap_or_default(),
            None => String::new(),
        },
    };
    let content_html = markdown_service::render(&content);
    let title: Option<String> = dto.title.clone().or_else(|| existing.title.clone());
    let transcript = existing.transcript.clone().unwrap_or_default();

    // Écriture du fichier (créé si absent).
    let file_id = match existing.file_id {
        Some(fid) => { content_files::write_note(state, owner_id, fid, &content, &content_html).await.map_err(|e| anyhow::anyhow!(e))?; fid }
        None => content_files::create_note_file(state, owner_id, title.as_deref(), &content, &content_html).await.map_err(|e| anyhow::anyhow!(e))?,
    };
    let preview    = content_files::make_preview(&content);
    let word_count = content.split_whitespace().count() as i32;

    let mut note = sqlx::query_as::<_, Note>(
        r#"UPDATE notes
           SET title        = COALESCE($1, title),
               file_id      = $2,
               preview      = $3,
               color        = COALESCE($4, color),
               checklist    = COALESCE($5, checklist),
               notebook_id  = CASE WHEN $6::boolean THEN $7 ELSE notebook_id END,
               is_pinned    = COALESCE($8, is_pinned),
               is_archived  = COALESCE($9, is_archived),
               word_count   = $10,
               search_vector =
                   setweight(to_tsvector('french', unaccent(COALESCE($11, ''))), 'A') ||
                   setweight(to_tsvector('french', unaccent($12)),               'B') ||
                   setweight(to_tsvector('french', unaccent($13)),               'C'),
               updated_at   = NOW()
           WHERE id = $14 AND owner_id = $15
           RETURNING *"#,
    )
    .bind(dto.title.as_deref())   // $1
    .bind(file_id)                // $2
    .bind(&preview)               // $3
    .bind(dto.color.as_deref())   // $4
    .bind(dto.checklist.as_ref()) // $5
    .bind(dto.notebook_id.is_some()) // $6
    .bind(dto.notebook_id)        // $7
    .bind(dto.is_pinned)          // $8
    .bind(dto.is_archived)        // $9
    .bind(word_count)             // $10
    .bind(title.as_deref())       // $11  titre effectif (search A)
    .bind(&content)               // $12  contenu (search B)
    .bind(&transcript)            // $13  transcript (search C)
    .bind(id)                     // $14
    .bind(owner_id)               // $15
    .fetch_optional(&state.db)
    .await
    .context("update_note")?
    .ok_or_else(|| anyhow::anyhow!("note disparue"))?;

    note.content = content;
    note.content_html = Some(content_html);
    Ok(Some(note))
}

pub async fn trash_note(db: &sqlx::PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = sqlx::query(
        "UPDATE notes SET is_trashed = TRUE, trashed_at = NOW(), updated_at = NOW() WHERE id = $1 AND owner_id = $2 AND is_trashed = FALSE",
    )
    .bind(id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("trash_note")?
    .rows_affected();
    Ok(rows > 0)
}

pub async fn restore_note(db: &sqlx::PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = sqlx::query(
        "UPDATE notes SET is_trashed = FALSE, trashed_at = NULL, updated_at = NOW() WHERE id = $1 AND owner_id = $2 AND is_trashed = TRUE",
    )
    .bind(id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("restore_note")?
    .rows_affected();
    Ok(rows > 0)
}

pub async fn delete_note(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<bool> {
    // Supprime la ligne et le fichier de contenu associé.
    let file_id: Option<Option<Uuid>> = sqlx::query_scalar(
        "DELETE FROM notes WHERE id = $1 AND owner_id = $2 AND is_trashed = TRUE RETURNING file_id",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await
    .context("delete_note")?;

    match file_id {
        Some(fid) => {
            if let Some(fid) = fid { content_files::delete_note_file(state, owner_id, fid).await; }
            Ok(true)
        }
        None => Ok(false),
    }
}

pub async fn empty_trash(state: &AppState, owner_id: Uuid) -> Result<u64> {
    let file_ids: Vec<Option<Uuid>> = sqlx::query_scalar(
        "DELETE FROM notes WHERE owner_id = $1 AND is_trashed = TRUE RETURNING file_id",
    )
    .bind(owner_id)
    .fetch_all(&state.db)
    .await
    .context("empty_trash")?;

    let count = file_ids.len() as u64;
    for fid in file_ids.into_iter().flatten() {
        content_files::delete_note_file(state, owner_id, fid).await;
    }
    Ok(count)
}

pub async fn duplicate_note(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<Option<Note>> {
    // Note source (métadonnée).
    let src = match sqlx::query_as::<_, Note>(
        "SELECT * FROM notes WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.db)
    .await
    .context("duplicate_note fetch")?
    {
        Some(n) => n,
        None => return Ok(None),
    };

    // Contenu source depuis le fichier.
    let (content, content_html) = match src.file_id {
        Some(fid) => content_files::read_note(state, owner_id, fid).await.unwrap_or_default(),
        None => (String::new(), String::new()),
    };
    let new_title = src.title.as_ref().map(|t| format!("{t} (copie)"));
    let file_id   = content_files::create_note_file(state, owner_id, new_title.as_deref(), &content, &content_html)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    let preview    = content_files::make_preview(&content);
    let word_count = content.split_whitespace().count() as i32;
    let transcript = src.transcript.clone().unwrap_or_default();

    let mut note = sqlx::query_as::<_, Note>(
        r#"INSERT INTO notes
           (owner_id, notebook_id, title, file_id, preview, note_type, color, checklist, word_count, search_vector)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
               setweight(to_tsvector('french', unaccent(COALESCE($3, ''))), 'A') ||
               setweight(to_tsvector('french', unaccent($10)),              'B') ||
               setweight(to_tsvector('french', unaccent($11)),              'C'))
           RETURNING *"#,
    )
    .bind(owner_id)
    .bind(src.notebook_id)
    .bind(new_title.as_deref())
    .bind(file_id)
    .bind(&preview)
    .bind(&src.note_type)
    .bind(&src.color)
    .bind(&src.checklist)
    .bind(word_count)
    .bind(&content)
    .bind(&transcript)
    .fetch_one(&state.db)
    .await
    .context("duplicate_note")?;

    note.content = content;
    note.content_html = Some(content_html);
    Ok(Some(note))
}
