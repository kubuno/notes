use reqwest::Client;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::events;

struct DueReminder {
    id:         Uuid,
    note_id:    Uuid,
    owner_id:   Uuid,
    fire_at:    chrono::DateTime<chrono::Utc>,
    method:     String,
    recurrence: Option<String>,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for DueReminder {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id:         row.try_get("id")?,
            note_id:    row.try_get("note_id")?,
            owner_id:   row.try_get("owner_id")?,
            fire_at:    row.try_get("fire_at")?,
            method:     row.try_get("method")?,
            recurrence: row.try_get("recurrence")?,
        })
    }
}

pub async fn start(db: PgPool, http: Client, core_url: String, secret: String, interval_s: u64) {
    loop {
        match check_and_send(&db, &http, &core_url, &secret).await {
            Ok(sent) if sent > 0 => tracing::info!("Rappels envoyés : {sent}"),
            Err(e) => tracing::error!(error = %e, "Worker rappels"),
            _ => {}
        }
        tokio::time::sleep(Duration::from_secs(interval_s)).await;
    }
}

async fn check_and_send(
    db:     &PgPool,
    http:   &Client,
    core_url: &str,
    secret:   &str,
) -> anyhow::Result<usize> {
    let due = sqlx::query_as::<_, DueReminder>(
        "SELECT id, note_id, owner_id, fire_at, method, recurrence
         FROM reminders
         WHERE fire_at <= NOW() AND sent_at IS NULL
         ORDER BY fire_at
         LIMIT 50
         FOR UPDATE SKIP LOCKED",
    )
    .fetch_all(db)
    .await?;

    let count = due.len();

    for r in &due {
        let ev = serde_json::json!({
            "type": "ReminderFired",
            "payload": {
                "user_id":   r.owner_id,
                "note_id":   r.note_id,
                "method":    r.method,
                "module_id": "notes",
            }
        });
        let _ = events::publish_event(http, core_url, secret, ev).await;

        let next = r.recurrence.as_deref().and_then(|rec| match rec {
            "daily"  => Some(r.fire_at + chrono::Duration::days(1)),
            "weekly" => Some(r.fire_at + chrono::Duration::weeks(1)),
            _        => None,
        });

        if let Some(next_at) = next {
            sqlx::query("UPDATE reminders SET fire_at = $1 WHERE id = $2")
                .bind(next_at)
                .bind(r.id)
                .execute(db)
                .await?;
        } else {
            sqlx::query("UPDATE reminders SET sent_at = NOW() WHERE id = $1")
                .bind(r.id)
                .execute(db)
                .await?;
        }
    }

    Ok(count)
}
