use anyhow::{Context, Result};
use kubuno_db::search::{self, Query};
use kubuno_db::{new_id, params, DbPool, DbValue};
use uuid::Uuid;

use crate::models::{CreateNoteDto, ListNotesQuery, Note, UpdateNoteDto};
use crate::services::notebook_service;
use crate::services::{content_files, markdown_service, search_service};
use crate::state::AppState;
use crate::sync;

/// The three normalized search columns of a note, computed in Rust at write time
/// (title = weight A, body = B, transcript = C). See `kubuno_db::search`.
fn norms(title: Option<&str>, content: &str, transcript: &str) -> (String, String, String) {
    (
        search::normalize(title.unwrap_or("")),
        search::normalize(content),
        search::normalize(transcript),
    )
}

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
    let limit = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);
    let trashed = q.trashed.unwrap_or(false);

    // Full-text search branch (normalized columns + portable LIKE).
    if let Some(ref term) = q.search {
        // Pre-conditions: $1 = owner_id, $2 = is_trashed. Search binds from $3.
        let Some(s) = Query::build(term, &search_service::fields(), 3) else {
            return Ok(Vec::new());
        };
        let sql = format!(
            "SELECT * FROM notes.notes \
             WHERE owner_id = $1 AND is_trashed = $2 AND {where_sql} \
             ORDER BY {order_sql} DESC, updated_at DESC \
             LIMIT ${limit_ph} OFFSET ${offset_ph}",
            where_sql = s.where_sql,
            order_sql = s.order_sql,
            limit_ph = s.next,
            offset_ph = s.next + 1,
        );
        let mut binds = params![owner_id, trashed];
        binds.extend(s.binds);
        binds.push(limit.into());
        binds.push(offset.into());
        let notes = db
            .fetch_all_as::<Note>(&sql, binds)
            .await
            .context("list_notes search")?;
        return Ok(fill_preview(notes));
    }

    // Plain listing: build the WHERE dynamically so each optional filter binds a
    // fresh, strictly-increasing placeholder (the rewriter forbids reusing one,
    // and `$n::type IS NULL` casts are PostgreSQL-only anyway).
    let mut binds: Vec<DbValue> = params![owner_id, trashed];
    let mut wheres = vec!["owner_id = $1".to_string(), "is_trashed = $2".to_string()];
    let mut n = 3usize;
    if let Some(nb) = q.notebook_id {
        wheres.push(format!("notebook_id = ${n}"));
        binds.push(nb.into());
        n += 1;
    }
    if let Some(ref t) = q.note_type {
        wheres.push(format!("note_type = ${n}"));
        binds.push(t.as_str().into());
        n += 1;
    }
    if let Some(p) = q.pinned {
        wheres.push(format!("is_pinned = ${n}"));
        binds.push(p.into());
        n += 1;
    }
    if let Some(a) = q.archived {
        wheres.push(format!("is_archived = ${n}"));
        binds.push(a.into());
        n += 1;
    }
    if let Some(lid) = q.label_id {
        wheres.push(format!(
            "id IN (SELECT note_id FROM notes.note_labels WHERE label_id = ${n})"
        ));
        binds.push(lid.into());
        n += 1;
    }
    let limit_ph = n;
    let offset_ph = n + 1;
    binds.push(limit.into());
    binds.push(offset.into());

    let sql = format!(
        "SELECT * FROM notes.notes WHERE {} \
         ORDER BY is_pinned DESC, updated_at DESC \
         LIMIT ${limit_ph} OFFSET ${offset_ph}",
        wheres.join(" AND ")
    );
    let notes = db
        .fetch_all_as::<Note>(&sql, binds)
        .await
        .context("list_notes")?;
    Ok(fill_preview(notes))
}

/// Récupère la note avec son contenu complet (lu depuis le fichier .kbnot).
pub async fn get_note(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<Option<Note>> {
    let mut note = match state
        .db
        .fetch_optional_as::<Note>(
            "SELECT * FROM notes.notes WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
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
    let id = dto.id.unwrap_or_else(new_id);
    let content = dto.content.unwrap_or_default();
    let content_html = markdown_service::render(&content);
    let note_type = dto.note_type.as_deref().unwrap_or("text").to_string();
    let color = dto.color.as_deref().unwrap_or("default").to_string();
    let checklist = dto.checklist.unwrap_or(serde_json::json!([]));
    let title = dto.title.clone();
    let notebook_id = dto.notebook_id;
    let is_pinned = dto.is_pinned.unwrap_or(false);

    // Contenu → fichier .kbnot.
    let file_id =
        content_files::create_note_file(state, owner_id, title.as_deref(), &content, &content_html)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    let preview = content_files::make_preview(&content);
    let word_count = content.split_whitespace().count() as i32;
    let (title_norm, body_norm, transcript_norm) = norms(title.as_deref(), &content, "");
    let now = chrono::Utc::now();

    let mut tx = state.db.begin().await.context("create_note begin")?;
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    tx.execute(
        "INSERT INTO notes.notes \
            (id, owner_id, notebook_id, title, note_type, color, checklist, mentions, mentioned_by, \
             is_pinned, file_id, preview, word_count, title_norm, body_norm, transcript_norm, \
             change_seq, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
        params![
            id, owner_id, notebook_id, title.as_deref(), &note_type, &color, checklist,
            Vec::<Uuid>::new(), Vec::<Uuid>::new(), is_pinned, file_id, &preview, word_count,
            &title_norm, &body_norm, &transcript_norm, seq, now, now
        ],
    )
    .await
    .context("create_note insert")?;
    if let Some(nb) = notebook_id {
        notebook_service::adjust_note_count(&mut tx, nb, 1)
            .await
            .context("create_note count")?;
    }
    tx.commit().await.context("create_note commit")?;

    let mut note = reselect(state, id, owner_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("note disparue"))?;
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
    // Note existante (métadonnée : file_id, title, transcript, notebook_id).
    let existing = match reselect(state, id, owner_id).await? {
        Some(n) => n,
        None => return Ok(None),
    };

    // Contenu : nouveau si fourni, sinon lecture du fichier existant.
    let content: String = match dto.content.clone() {
        Some(c) => c,
        None => match existing.file_id {
            Some(fid) => content_files::read_note(state, owner_id, fid)
                .await
                .map(|(c, _)| c)
                .unwrap_or_default(),
            None => String::new(),
        },
    };
    let content_html = markdown_service::render(&content);
    let title: Option<String> = dto.title.clone().or_else(|| existing.title.clone());
    let transcript = existing.transcript.clone().unwrap_or_default();

    // Écriture du fichier (créé si absent).
    let file_id = match existing.file_id {
        Some(fid) => {
            content_files::write_note(state, owner_id, fid, &content, &content_html)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            fid
        }
        None => content_files::create_note_file(state, owner_id, title.as_deref(), &content, &content_html)
            .await
            .map_err(|e| anyhow::anyhow!(e))?,
    };
    let preview = content_files::make_preview(&content);
    let word_count = content.split_whitespace().count() as i32;
    let (title_norm, body_norm, transcript_norm) = norms(title.as_deref(), &content, &transcript);
    let now = chrono::Utc::now();

    // Notebook change (the CASE below only moves it when the DTO provides one).
    let notebook_changes = dto.notebook_id.is_some();
    let new_notebook = dto.notebook_id;

    let mut tx = state.db.begin().await.context("update_note begin")?;
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    let affected = tx
        .execute(
            "UPDATE notes.notes \
             SET title           = COALESCE($1, title), \
                 file_id         = $2, \
                 preview         = $3, \
                 color           = COALESCE($4, color), \
                 checklist       = COALESCE($5, checklist), \
                 notebook_id     = CASE WHEN $6 THEN $7 ELSE notebook_id END, \
                 is_pinned       = COALESCE($8, is_pinned), \
                 is_archived     = COALESCE($9, is_archived), \
                 word_count      = $10, \
                 title_norm      = $11, \
                 body_norm       = $12, \
                 transcript_norm = $13, \
                 change_seq      = $14, \
                 updated_at      = $15 \
             WHERE id = $16 AND owner_id = $17",
            params![
                dto.title.as_deref(),      // $1
                file_id,                   // $2
                &preview,                  // $3
                dto.color.as_deref(),      // $4
                dto.checklist.clone(),     // $5
                notebook_changes,          // $6
                new_notebook,              // $7
                dto.is_pinned,             // $8
                dto.is_archived,           // $9
                word_count,                // $10
                &title_norm,               // $11
                &body_norm,                // $12
                &transcript_norm,          // $13
                seq,                       // $14
                now,                       // $15
                id,                        // $16
                owner_id                   // $17
            ],
        )
        .await
        .context("update_note")?;
    if affected == 0 {
        tx.rollback().await.ok();
        return Ok(None);
    }
    // Maintain notebook counts when the note moved.
    if notebook_changes && new_notebook != existing.notebook_id {
        if let Some(old_nb) = existing.notebook_id {
            notebook_service::adjust_note_count(&mut tx, old_nb, -1)
                .await
                .context("update_note dec")?;
        }
        if let Some(new_nb) = new_notebook {
            notebook_service::adjust_note_count(&mut tx, new_nb, 1)
                .await
                .context("update_note inc")?;
        }
    }
    tx.commit().await.context("update_note commit")?;

    let mut note = reselect(state, id, owner_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("note disparue"))?;
    note.content = content;
    note.content_html = Some(content_html);
    Ok(Some(note))
}

pub async fn trash_note(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let now = chrono::Utc::now();
    let mut tx = db.begin().await.context("trash_note begin")?;
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    let rows = tx
        .execute(
            "UPDATE notes.notes \
             SET is_trashed = $1, trashed_at = $2, updated_at = $3, change_seq = $4 \
             WHERE id = $5 AND owner_id = $6 AND is_trashed = $7",
            params![true, now, now, seq, id, owner_id, false],
        )
        .await
        .context("trash_note")?;
    tx.commit().await.context("trash_note commit")?;
    Ok(rows > 0)
}

pub async fn restore_note(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let now = chrono::Utc::now();
    let mut tx = db.begin().await.context("restore_note begin")?;
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    let rows = tx
        .execute(
            "UPDATE notes.notes \
             SET is_trashed = $1, trashed_at = NULL, updated_at = $2, change_seq = $3 \
             WHERE id = $4 AND owner_id = $5 AND is_trashed = $6",
            params![false, now, seq, id, owner_id, true],
        )
        .await
        .context("restore_note")?;
    tx.commit().await.context("restore_note commit")?;
    Ok(rows > 0)
}

pub async fn delete_note(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<bool> {
    // Metadata needed for cleanup + count, read before the delete.
    let meta = state
        .db
        .fetch_optional_as::<DeleteMeta>(
            "SELECT file_id, notebook_id FROM notes.notes \
             WHERE id = $1 AND owner_id = $2 AND is_trashed = $3",
            params![id, owner_id, true],
        )
        .await
        .context("delete_note fetch")?;
    let Some(meta) = meta else { return Ok(false) };

    let mut tx = state.db.begin().await.context("delete_note begin")?;
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    sync::record_note_tombstone(&mut tx, id, owner_id, seq)
        .await
        .context("delete_note tombstone")?;
    tx.execute(
        "DELETE FROM notes.notes WHERE id = $1 AND owner_id = $2 AND is_trashed = $3",
        params![id, owner_id, true],
    )
    .await
    .context("delete_note")?;
    if let Some(nb) = meta.notebook_id {
        notebook_service::adjust_note_count(&mut tx, nb, -1)
            .await
            .context("delete_note count")?;
    }
    tx.commit().await.context("delete_note commit")?;

    if let Some(fid) = meta.file_id {
        content_files::delete_note_file(state, owner_id, fid).await;
    }
    Ok(true)
}

pub async fn empty_trash(state: &AppState, owner_id: Uuid) -> Result<u64> {
    let rows = state
        .db
        .fetch_all_as::<TrashRow>(
            "SELECT id, file_id, notebook_id FROM notes.notes \
             WHERE owner_id = $1 AND is_trashed = $2",
            params![owner_id, true],
        )
        .await
        .context("empty_trash fetch")?;

    let count = rows.len() as u64;
    if rows.is_empty() {
        return Ok(0);
    }

    let mut tx = state.db.begin().await.context("empty_trash begin")?;
    for r in &rows {
        let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
        sync::record_note_tombstone(&mut tx, r.id, owner_id, seq)
            .await
            .context("empty_trash tombstone")?;
        tx.execute(
            "DELETE FROM notes.notes WHERE id = $1 AND owner_id = $2",
            params![r.id, owner_id],
        )
        .await
        .context("empty_trash delete")?;
        if let Some(nb) = r.notebook_id {
            notebook_service::adjust_note_count(&mut tx, nb, -1)
                .await
                .context("empty_trash count")?;
        }
    }
    tx.commit().await.context("empty_trash commit")?;

    for r in rows {
        if let Some(fid) = r.file_id {
            content_files::delete_note_file(state, owner_id, fid).await;
        }
    }
    Ok(count)
}

pub async fn duplicate_note(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<Option<Note>> {
    let src = match state
        .db
        .fetch_optional_as::<Note>(
            "SELECT * FROM notes.notes WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("duplicate_note fetch")?
    {
        Some(n) => n,
        None => return Ok(None),
    };

    let (content, content_html) = match src.file_id {
        Some(fid) => content_files::read_note(state, owner_id, fid)
            .await
            .unwrap_or_default(),
        None => (String::new(), String::new()),
    };
    let new_id_ = new_id();
    let new_title = src.title.as_ref().map(|t| format!("{t} (copie)"));
    let file_id =
        content_files::create_note_file(state, owner_id, new_title.as_deref(), &content, &content_html)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    let preview = content_files::make_preview(&content);
    let word_count = content.split_whitespace().count() as i32;
    let transcript = src.transcript.clone().unwrap_or_default();
    let (title_norm, body_norm, transcript_norm) = norms(new_title.as_deref(), &content, &transcript);
    let now = chrono::Utc::now();

    let mut tx = state.db.begin().await.context("duplicate_note begin")?;
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    tx.execute(
        "INSERT INTO notes.notes \
            (id, owner_id, notebook_id, title, note_type, color, checklist, mentions, mentioned_by, \
             is_pinned, file_id, preview, word_count, title_norm, body_norm, transcript_norm, \
             change_seq, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
        params![
            new_id_, owner_id, src.notebook_id, new_title.as_deref(), &src.note_type, &src.color,
            src.checklist.clone(), Vec::<Uuid>::new(), Vec::<Uuid>::new(), false, file_id,
            &preview, word_count, &title_norm, &body_norm, &transcript_norm, seq, now, now
        ],
    )
    .await
    .context("duplicate_note insert")?;
    if let Some(nb) = src.notebook_id {
        notebook_service::adjust_note_count(&mut tx, nb, 1)
            .await
            .context("duplicate_note count")?;
    }
    tx.commit().await.context("duplicate_note commit")?;

    let mut note = reselect(state, new_id_, owner_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("note disparue"))?;
    note.content = content;
    note.content_html = Some(content_html);
    Ok(Some(note))
}

async fn reselect(state: &AppState, id: Uuid, owner_id: Uuid) -> Result<Option<Note>> {
    state
        .db
        .fetch_optional_as::<Note>(
            "SELECT * FROM notes.notes WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("reselect note")
}

#[derive(sqlx::FromRow)]
struct DeleteMeta {
    file_id: Option<Uuid>,
    notebook_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct TrashRow {
    id: Uuid,
    file_id: Option<Uuid>,
    notebook_id: Option<Uuid>,
}
