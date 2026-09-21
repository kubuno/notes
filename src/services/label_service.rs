use anyhow::{Context, Result};
use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::models::{CreateLabelDto, Label, UpdateLabelDto};
use crate::sync;

#[derive(sqlx::FromRow)]
struct IdRow {
    id: Uuid,
}

pub async fn list_labels(db: &DbPool, owner_id: Uuid) -> Result<Vec<Label>> {
    let labels = db
        .fetch_all_as::<Label>(
            "SELECT * FROM notes.labels WHERE owner_id = $1 ORDER BY position, name",
            params![owner_id],
        )
        .await
        .context("list_labels")?;
    Ok(labels)
}

pub async fn create_label(db: &DbPool, owner_id: Uuid, dto: CreateLabelDto) -> Result<Label> {
    let id = dto.id.unwrap_or_else(new_id);
    let color = dto.color.as_deref().unwrap_or("#1a73e8");
    let position = dto.position.unwrap_or(0);
    let now = chrono::Utc::now();

    let mut tx = db.begin().await.context("create_label begin")?;
    let seq = sync::next_label_seq(&mut tx).await.context("label seq")?;
    tx.execute(
        "INSERT INTO notes.labels (id, owner_id, name, color, position, change_seq, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        params![id, owner_id, &dto.name, color, position, seq, now],
    )
    .await
    .context("create_label insert")?;
    tx.commit().await.context("create_label commit")?;

    reselect_label(db, id, owner_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("label disparu"))
}

pub async fn update_label(
    db: &DbPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateLabelDto,
) -> Result<Option<Label>> {
    let mut tx = db.begin().await.context("update_label begin")?;
    let seq = sync::next_label_seq(&mut tx).await.context("label seq")?;
    let affected = tx
        .execute(
            "UPDATE notes.labels \
             SET name       = COALESCE($1, name), \
                 color      = COALESCE($2, color), \
                 position   = COALESCE($3, position), \
                 change_seq = $4 \
             WHERE id = $5 AND owner_id = $6",
            params![dto.name.as_deref(), dto.color.as_deref(), dto.position, seq, id, owner_id],
        )
        .await
        .context("update_label")?;
    tx.commit().await.context("update_label commit")?;

    if affected == 0 {
        return Ok(None);
    }
    reselect_label(db, id, owner_id).await
}

pub async fn delete_label(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let owns: Option<Uuid> = db
        .fetch_optional_scalar::<Uuid>(
            "SELECT id FROM notes.labels WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("delete_label check")?;
    if owns.is_none() {
        return Ok(false);
    }

    // The FK cascades to note_labels; the old `note_labels` bump trigger fired for
    // each such deletion, so the notes that carried this label must be bumped.
    let note_ids: Vec<Uuid> = db
        .fetch_all_as::<IdRow>(
            "SELECT note_id AS id FROM notes.note_labels WHERE label_id = $1",
            params![id],
        )
        .await
        .context("delete_label affected notes")?
        .into_iter()
        .map(|r| r.id)
        .collect();

    let mut tx = db.begin().await.context("delete_label begin")?;
    for nid in &note_ids {
        sync::touch_note(&mut tx, *nid).await.context("bump note")?;
    }
    let lseq = sync::next_label_seq(&mut tx).await.context("label seq")?;
    sync::record_label_tombstone(&mut tx, id, owner_id, lseq)
        .await
        .context("delete_label tombstone")?;
    tx.execute("DELETE FROM notes.labels WHERE id = $1", params![id])
        .await
        .context("delete_label")?;
    tx.commit().await.context("delete_label commit")?;
    Ok(true)
}

pub async fn assign_label(db: &DbPool, note_id: Uuid, label_id: Uuid, owner_id: Uuid) -> Result<bool> {
    let note_exists: Option<Uuid> = db
        .fetch_optional_scalar::<Uuid>(
            "SELECT id FROM notes.notes WHERE id = $1 AND owner_id = $2",
            params![note_id, owner_id],
        )
        .await
        .context("assign_label: check note")?;
    if note_exists.is_none() {
        return Ok(false);
    }

    let backend = db.backend();
    let ignore = backend.insert_ignore_prefix();
    let on_conflict = backend.on_conflict_do_nothing(&["note_id", "label_id"]);
    let now = chrono::Utc::now();
    let sql = format!(
        "INSERT {ignore}INTO notes.note_labels (note_id, label_id, created_at) \
         VALUES ($1, $2, $3){on_conflict}"
    );

    let mut tx = db.begin().await.context("assign_label begin")?;
    let inserted = tx
        .execute(&sql, params![note_id, label_id, now])
        .await
        .context("assign_label")?;
    // A no-op on conflict (label already assigned) is not a note change.
    if inserted > 0 {
        sync::touch_note(&mut tx, note_id).await.context("bump note")?;
    }
    tx.commit().await.context("assign_label commit")?;
    Ok(true)
}

pub async fn remove_label(db: &DbPool, note_id: Uuid, label_id: Uuid, owner_id: Uuid) -> Result<bool> {
    let mut tx = db.begin().await.context("remove_label begin")?;
    // note_id is bound twice (the placeholder rewriter forbids reusing $n).
    let removed = tx
        .execute(
            "DELETE FROM notes.note_labels \
             WHERE note_id = $1 AND label_id = $2 \
               AND (SELECT owner_id FROM notes.notes WHERE id = $3) = $4",
            params![note_id, label_id, note_id, owner_id],
        )
        .await
        .context("remove_label")?;
    if removed > 0 {
        sync::touch_note(&mut tx, note_id).await.context("bump note")?;
    }
    tx.commit().await.context("remove_label commit")?;
    Ok(removed > 0)
}

pub async fn list_note_labels(db: &DbPool, note_id: Uuid) -> Result<Vec<Label>> {
    let labels = db
        .fetch_all_as::<Label>(
            "SELECT l.* FROM notes.labels l \
             INNER JOIN notes.note_labels nl ON nl.label_id = l.id \
             WHERE nl.note_id = $1 \
             ORDER BY l.position, l.name",
            params![note_id],
        )
        .await
        .context("list_note_labels")?;
    Ok(labels)
}

async fn reselect_label(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<Option<Label>> {
    db.fetch_optional_as::<Label>(
        "SELECT * FROM notes.labels WHERE id = $1 AND owner_id = $2",
        params![id, owner_id],
    )
    .await
    .context("reselect_label")
}
