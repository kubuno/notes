use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateNotebookDto, Notebook, UpdateNotebookDto};

pub async fn list_notebooks(db: &PgPool, owner_id: Uuid) -> Result<Vec<Notebook>> {
    let notebooks = sqlx::query_as::<_, Notebook>(
        "SELECT * FROM notebooks WHERE owner_id = $1 ORDER BY position, name",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .context("list_notebooks")?;
    Ok(notebooks)
}

pub async fn create_notebook(
    db: &PgPool,
    owner_id: Uuid,
    dto: CreateNotebookDto,
) -> Result<Notebook> {
    let icon = dto.icon.as_deref().unwrap_or("📁");
    let notebook = sqlx::query_as::<_, Notebook>(
        r#"INSERT INTO notebooks (owner_id, parent_id, name, icon, color)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *"#,
    )
    .bind(owner_id)
    .bind(dto.parent_id)
    .bind(&dto.name)
    .bind(icon)
    .bind(dto.color.as_deref())
    .fetch_one(db)
    .await
    .context("create_notebook")?;
    Ok(notebook)
}

pub async fn update_notebook(
    db: &PgPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateNotebookDto,
) -> Result<Option<Notebook>> {
    let notebook = sqlx::query_as::<_, Notebook>(
        r#"UPDATE notebooks
           SET name      = COALESCE($1, name),
               icon      = COALESCE($2, icon),
               color     = COALESCE($3, color),
               position  = COALESCE($4, position),
               updated_at = NOW()
           WHERE id = $5 AND owner_id = $6
           RETURNING *"#,
    )
    .bind(dto.name.as_deref())
    .bind(dto.icon.as_deref())
    .bind(dto.color.as_deref())
    .bind(dto.position)
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("update_notebook")?;
    Ok(notebook)
}

pub async fn delete_notebook(db: &PgPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = sqlx::query(
        "DELETE FROM notebooks WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("delete_notebook")?
    .rows_affected();
    Ok(rows > 0)
}
