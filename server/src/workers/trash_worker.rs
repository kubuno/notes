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
//! drive, so each purged row's file is removed too. A sync tombstone is written
//! in the same transaction as each delete (the portable replacement for the old
//! `AFTER DELETE` trigger), so offline clients drop the note on their next pull
//! instead of resurrecting it.

use std::time::Duration;
use uuid::Uuid;

use kubuno_db::params;

use crate::services::{content_files, notebook_service};
use crate::state::AppState;
use crate::sync;

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
    // The cutoff is computed in Rust (portable: no `make_interval` / date maths in
    // SQL). Candidates are listed first, then each is deleted with its tombstone
    // and notebook-count adjustment in one transaction — the delta layer is the
    // journal now, not a trigger, so the tombstone is written explicitly.
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let rows = state
        .db
        .fetch_all_as::<PurgeRow>(
            "SELECT id, owner_id, file_id, notebook_id FROM notes.notes \
             WHERE is_trashed = $1 AND trashed_at IS NOT NULL AND trashed_at < $2 \
             ORDER BY trashed_at \
             LIMIT $3",
            params![true, cutoff, BATCH],
        )
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Lecture des notes expirées de la corbeille"))?;

    if rows.is_empty() {
        return Ok(0);
    }

    let mut tx = state.db.begin().await?;
    for r in &rows {
        let seq = sync::next_note_seq(&mut tx).await?;
        sync::record_note_tombstone(&mut tx, r.id, r.owner_id, seq).await?;
        tx.execute(
            "DELETE FROM notes.notes WHERE id = $1 AND is_trashed = $2",
            params![r.id, true],
        )
        .await?;
        if let Some(nb) = r.notebook_id {
            notebook_service::adjust_note_count(&mut tx, nb, -1).await?;
        }
    }
    tx.commit().await?;

    let purged = rows.len();
    for r in rows {
        if let Some(fid) = r.file_id {
            content_files::delete_note_file(state, r.owner_id, fid).await;
        }
    }
    Ok(purged)
}

#[derive(sqlx::FromRow)]
struct PurgeRow {
    id: Uuid,
    owner_id: Uuid,
    file_id: Option<Uuid>,
    notebook_id: Option<Uuid>,
}
