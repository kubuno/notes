//! Trash cleaner: purges notes that have sat in the bin longer than the
//! instance allows.
//!
//! A bin nobody empties is not a bin, it is a second archive that keeps counting
//! against the account's quota. The retention window comes from the admin
//! console (`notes.trash_retention_days`), is re-read on every pass, and `0`
//! means "never purge" — an administrator must be able to turn the sweep off
//! without stopping the module.
//!
//! Deleting the row is not enough: a note's body lives in a `.kbnot` file in the
//! drive, so each purged row's file is removed too. The `notes` DELETE trigger
//! writes the sync tombstone, so offline clients drop the note on their next
//! pull instead of resurrecting it.

use std::time::Duration;
use uuid::Uuid;

use crate::services::content_files;
use crate::state::AppState;

/// How often the bin is swept. Retention is measured in days, so an hourly pass
/// is precise enough and stays cheap.
const SWEEP_INTERVAL: Duration = Duration::from_secs(3600);

/// Rows purged per pass. Bounded so a bin left unswept for months cannot turn
/// one wake-up into a multi-minute transaction holding locks on `notes`.
const BATCH: i64 = 500;

pub async fn start(state: AppState) {
    loop {
        tokio::time::sleep(SWEEP_INTERVAL).await;

        let days = state.instance().trash_retention_days;
        if days <= 0 {
            continue; // retention disabled: the bin is kept forever
        }

        match sweep(&state, days).await {
            Ok(0) => {}
            Ok(n) => tracing::info!(purged = n, retention_days = days, "Corbeille des notes purgée"),
            Err(e) => tracing::error!(error = %e, "Purge de la corbeille des notes"),
        }
    }
}

/// One bounded pass. Returns how many notes were purged.
async fn sweep(state: &AppState, days: i32) -> Result<usize, sqlx::Error> {
    // `RETURNING` gives back exactly the rows this statement removed, so the
    // file cleanup below can never target a note someone else just restored.
    let rows = sqlx::query_as::<_, (Uuid, Option<Uuid>)>(
        r#"DELETE FROM notes
           WHERE id IN (
               SELECT id FROM notes
               WHERE is_trashed = TRUE
                 AND trashed_at IS NOT NULL
                 AND trashed_at < NOW() - make_interval(days => $1)
               ORDER BY trashed_at
               LIMIT $2
           )
           RETURNING owner_id, file_id"#,
    )
    .bind(days)
    .bind(BATCH)
    .fetch_all(&state.db)
    .await
    .inspect_err(|e| tracing::error!(error = %e, "Suppression des notes expirées de la corbeille"))?;

    let purged = rows.len();
    for (owner_id, file_id) in rows {
        if let Some(fid) = file_id {
            content_files::delete_note_file(state, owner_id, fid).await;
        }
    }
    Ok(purged)
}
