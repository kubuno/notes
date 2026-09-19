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
    let now = chrono::Utc::now();

    // List the candidates, then claim each one on its own. The claim is the
    // UPDATE itself: it only touches a reminder that is still unsent, so the
    // number of rows it changed IS the proof of ownership — exactly one worker
    // can see 1. Reading the candidates under `FOR UPDATE SKIP LOCKED` would
    // prove nothing here, because a statement run outside an explicit
    // transaction commits (and releases its row locks) the moment it returns.
    // The guard column is also the only form of this that any SQL engine can
    // express: neither SQLite nor MariaDB offers `SKIP LOCKED`.
    let due = sqlx::query_as::<_, DueReminder>(
        "SELECT id, note_id, owner_id, fire_at, method, recurrence
         FROM reminders
         WHERE fire_at <= $1 AND sent_at IS NULL
         ORDER BY fire_at
         LIMIT 50",
    )
    .bind(now)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Rappels : lecture des échéances impossible");
        e
    })?;

    let mut count = 0usize;

    for r in &due {
        // One statement both claims the reminder and leaves it in its final
        // state, so a crash can never strand a half-fired reminder: a
        // recurring one is moved to its next occurrence (its old date is part
        // of the guard), a one-shot one is stamped sent.
        let next = r.recurrence.as_deref().and_then(|rec| match rec {
            "daily"  => Some(r.fire_at + chrono::Duration::days(1)),
            "weekly" => Some(r.fire_at + chrono::Duration::weeks(1)),
            _        => None,
        });
        let claim = match next {
            Some(next_at) => sqlx::query(
                "UPDATE reminders SET fire_at = $2
                 WHERE id = $1 AND sent_at IS NULL AND fire_at = $3",
            )
            .bind(r.id)
            .bind(next_at)
            .bind(r.fire_at),
            None => sqlx::query(
                "UPDATE reminders SET sent_at = $2 WHERE id = $1 AND sent_at IS NULL",
            )
            .bind(r.id)
            .bind(now),
        };
        let claimed = claim
            .execute(db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, reminder_id = %r.id, "Rappels : réservation impossible");
                e
            })?
            .rows_affected();
        if claimed != 1 {
            // Another worker got there first, or the reminder was cancelled
            // between the two statements. Not ours to fire.
            continue;
        }
        count += 1;

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
    }

    Ok(count)
}
