use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateReminderDto, Reminder, UpdateReminderDto};

pub async fn list_reminders(db: &PgPool, note_id: Uuid, owner_id: Uuid) -> Result<Vec<Reminder>> {
    let reminders = sqlx::query_as::<_, Reminder>(
        "SELECT * FROM reminders WHERE note_id = $1 AND owner_id = $2 ORDER BY fire_at",
    )
    .bind(note_id)
    .bind(owner_id)
    .fetch_all(db)
    .await
    .context("list_reminders")?;
    Ok(reminders)
}

pub async fn create_reminder(
    db: &PgPool,
    note_id: Uuid,
    owner_id: Uuid,
    dto: CreateReminderDto,
) -> Result<Reminder> {
    let method     = dto.method.as_deref().unwrap_or("notification");
    let recurrence = dto.recurrence.as_deref().unwrap_or("once");

    let reminder = sqlx::query_as::<_, Reminder>(
        r#"INSERT INTO reminders (note_id, owner_id, fire_at, method, recurrence)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *"#,
    )
    .bind(note_id)
    .bind(owner_id)
    .bind(dto.fire_at)
    .bind(method)
    .bind(recurrence)
    .fetch_one(db)
    .await
    .context("create_reminder")?;
    Ok(reminder)
}

pub async fn update_reminder(
    db: &PgPool,
    id: Uuid,
    note_id: Uuid,
    owner_id: Uuid,
    dto: UpdateReminderDto,
) -> Result<Option<Reminder>> {
    let reminder = sqlx::query_as::<_, Reminder>(
        r#"UPDATE reminders
           SET fire_at    = COALESCE($1, fire_at),
               method     = COALESCE($2, method),
               recurrence = COALESCE($3, recurrence),
               sent_at    = NULL
           WHERE id = $4 AND note_id = $5 AND owner_id = $6
           RETURNING *"#,
    )
    .bind(dto.fire_at)
    .bind(dto.method.as_deref())
    .bind(dto.recurrence.as_deref())
    .bind(id)
    .bind(note_id)
    .bind(owner_id)
    .fetch_optional(db)
    .await
    .context("update_reminder")?;
    Ok(reminder)
}

pub async fn delete_reminder(
    db: &PgPool,
    id: Uuid,
    note_id: Uuid,
    owner_id: Uuid,
) -> Result<bool> {
    let rows = sqlx::query(
        "DELETE FROM reminders WHERE id = $1 AND note_id = $2 AND owner_id = $3",
    )
    .bind(id)
    .bind(note_id)
    .bind(owner_id)
    .execute(db)
    .await
    .context("delete_reminder")?
    .rows_affected();
    Ok(rows > 0)
}
