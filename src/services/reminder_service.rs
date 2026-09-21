use anyhow::{Context, Result};
use kubuno_db::{new_id, params, DbPool};
use uuid::Uuid;

use crate::models::{CreateReminderDto, Reminder, UpdateReminderDto};

pub async fn list_reminders(db: &DbPool, note_id: Uuid, owner_id: Uuid) -> Result<Vec<Reminder>> {
    let reminders = db
        .fetch_all_as::<Reminder>(
            "SELECT * FROM notes.reminders WHERE note_id = $1 AND owner_id = $2 ORDER BY fire_at",
            params![note_id, owner_id],
        )
        .await
        .context("list_reminders")?;
    Ok(reminders)
}

pub async fn create_reminder(
    db: &DbPool,
    note_id: Uuid,
    owner_id: Uuid,
    dto: CreateReminderDto,
) -> Result<Reminder> {
    let id = new_id();
    let method = dto.method.as_deref().unwrap_or("notification");
    let recurrence = dto.recurrence.as_deref().unwrap_or("once");
    let now = chrono::Utc::now();

    // Reminders are not part of the delta sync, so no change_seq / journal here;
    // the id is generated in Rust and the row read back by id (no RETURNING).
    db.execute(
        "INSERT INTO notes.reminders (id, note_id, owner_id, fire_at, method, recurrence, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        params![id, note_id, owner_id, dto.fire_at, method, recurrence, now],
    )
    .await
    .context("create_reminder")?;

    db.fetch_one_as::<Reminder>("SELECT * FROM notes.reminders WHERE id = $1", params![id])
        .await
        .context("create_reminder reselect")
}

pub async fn update_reminder(
    db: &DbPool,
    id: Uuid,
    note_id: Uuid,
    owner_id: Uuid,
    dto: UpdateReminderDto,
) -> Result<Option<Reminder>> {
    // Confirm the reminder exists and belongs to this note/owner before writing,
    // so a MySQL re-select by id cannot resurrect a row a guard would have missed.
    let owns: Option<Uuid> = db
        .fetch_optional_scalar::<Uuid>(
            "SELECT id FROM notes.reminders WHERE id = $1 AND note_id = $2 AND owner_id = $3",
            params![id, note_id, owner_id],
        )
        .await
        .context("update_reminder check")?;
    if owns.is_none() {
        return Ok(None);
    }

    db.execute(
        "UPDATE notes.reminders \
         SET fire_at    = COALESCE($1, fire_at), \
             method     = COALESCE($2, method), \
             recurrence = COALESCE($3, recurrence), \
             sent_at    = NULL \
         WHERE id = $4 AND note_id = $5 AND owner_id = $6",
        params![dto.fire_at, dto.method.as_deref(), dto.recurrence.as_deref(), id, note_id, owner_id],
    )
    .await
    .context("update_reminder")?;

    db.fetch_optional_as::<Reminder>("SELECT * FROM notes.reminders WHERE id = $1", params![id])
        .await
        .context("update_reminder reselect")
}

pub async fn delete_reminder(db: &DbPool, id: Uuid, note_id: Uuid, owner_id: Uuid) -> Result<bool> {
    let rows = db
        .execute(
            "DELETE FROM notes.reminders WHERE id = $1 AND note_id = $2 AND owner_id = $3",
            params![id, note_id, owner_id],
        )
        .await
        .context("delete_reminder")?;
    Ok(rows > 0)
}
