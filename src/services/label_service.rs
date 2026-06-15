use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateLabelDto, Label, UpdateLabelDto};

pub async fn list_labels(db: &PgPool, owner_id: Uuid) -> Result<Vec<Label>> {
    let labels = sqlx::query_as::<_, Label>(
        "SELECT * FROM labels WHERE owner_id = $1 ORDER BY position, name",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .context("list_labels")?;
    Ok(labels)
}

pub async fn create_label(db: &PgPool, owner_id: Uuid, dto: CreateLabelDto) -> Result<Label> {
    let color = dto.color.as_deref().unwrap_or("#1a73e8");
    let label = sqlx::query_as::<_, Label>(
        r#"INSERT INTO labels (owner_id, name, color, position)
           VALUES ($1, $2, $3, $4)
           RETURNING *"#,
    )
    .bind(owner_id)
    .bind(&dto.name)
    .bind(color)
    .bind(dto.position.unwrap_or(0))
    .fetch_one(db)
    .await
    .context("create_label")?;
    Ok(label)
}

pub async fn update_label(
    db: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateLabelDto,
) -> Result<Option<Label>> {
    let label = sqlx::query_as::<_, Label>(
        r#"UPDATE labels
           SET name     = COALESCE($1, name),
               color    = COALESCE($2, color),
               position = COALESCE($3, position)
           WHERE id = $4 AND owner_id = $5
           RETURNING *"#,
    )
    .bind(dto.name.as_deref())
    .bind(dto.color.as_deref())
    .bind(dto.position)
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("update_label")?;
    Ok(label)
}

pub async fn delete_label(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM labels WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await
        .context("delete_label")?
        .rows_affected();
    Ok(rows > 0)
}

pub async fn assign_label(db: &PgPool, note_id: Uuid, label_id: Uuid, owner_id: Uuid) -> Result<bool> {
    // Vérifier ownership
    let note_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1 AND owner_id = $2)",
    )
    .bind(note_id)
    .bind(owner_id)
    .fetch_one(db)
    .await
    .context("assign_label: check note")?;

    if !note_exists { return Ok(false); }

    sqlx::query(
        "INSERT INTO note_labels (note_id, label_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(note_id)
    .bind(label_id)
    .execute(db)
    .await
    .context("assign_label")?;
    Ok(true)
}

pub async fn remove_label(db: &PgPool, note_id: Uuid, label_id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = sqlx::query(
        r#"DELETE FROM note_labels
           WHERE note_id = $1 AND label_id = $2
             AND (SELECT owner_id FROM notes WHERE id = $1) = $3"#,
    )
    .bind(note_id)
    .bind(label_id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("remove_label")?
    .rows_affected();
    Ok(rows > 0)
}

pub async fn list_note_labels(db: &PgPool, note_id: Uuid) -> Result<Vec<Label>> {
    let labels = sqlx::query_as::<_, Label>(
        r#"SELECT l.* FROM labels l
           INNER JOIN note_labels nl ON nl.label_id = l.id
           WHERE nl.note_id = $1
           ORDER BY l.position, l.name"#,
    )
    .bind(note_id)
    .fetch_all(db)
    .await
    .context("list_note_labels")?;
    Ok(labels)
}
