//! Delta-sync plumbing shared by the services and the delta handler.
//!
//! The local-first pull (notes / notebooks / labels) rests on a monotonic
//! `change_seq` per record and a tombstone per hard-deleted row. On PostgreSQL
//! that used to be a `SEQUENCE` plus `BEFORE UPDATE` / `AFTER DELETE` triggers;
//! here it is the portable [`kubuno_db::journal`] primitive, driven from Rust at
//! every write site. This module holds the literal table / domain names those
//! calls take — all `&'static str`, never request data — so the write sites read
//! uniformly and a rename happens in one place.
//!
//! Three entities are versioned: **notes**, **notebooks** and **labels**.
//!
//! * `note_labels` changes bump their **note** (label assignments ride inline in
//!   the note delta).
//! * The `notebooks.note_count` a note write maintains is a notebook change, so
//!   it takes a fresh notebook seq too.
//! * Notes are HARD-deleted (a trash is a soft `is_trashed` flag); a permanent
//!   delete writes a note tombstone. Notebooks and labels tombstone on delete.

use uuid::Uuid;

/// One shared counter table per schema; `next_seq` keys it by domain.
pub const CHANGE_COUNTER: &str = "notes.change_counter";

pub const NOTES_TABLE: &str = "notes.notes";
pub const NOTEBOOKS_TABLE: &str = "notes.notebooks";
pub const LABELS_TABLE: &str = "notes.labels";
pub const NOTE_TOMBSTONES: &str = "notes.note_tombstones";
pub const NOTEBOOK_TOMBSTONES: &str = "notes.notebook_tombstones";
pub const LABEL_TOMBSTONES: &str = "notes.label_tombstones";

/// Logical counter domains (the row keys in `change_counter`).
pub const NOTE_DOMAIN: &str = "notes";
pub const NOTEBOOK_DOMAIN: &str = "notebooks";
pub const LABEL_DOMAIN: &str = "labels";

/// The next monotonic sequence for the **notes** domain, taken inside `tx`.
pub async fn next_note_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, NOTE_DOMAIN).await
}

/// The next monotonic sequence for the **notebooks** domain, taken inside `tx`.
pub async fn next_notebook_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, NOTEBOOK_DOMAIN).await
}

/// The next monotonic sequence for the **labels** domain, taken inside `tx`.
pub async fn next_label_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, LABEL_DOMAIN).await
}

/// Bumps a **note** to a fresh sequence — the portable replacement for the old
/// child-triggered no-op `UPDATE`. Called after a label assignment changes.
/// Returns how many rows matched (`0` if the note is gone).
pub async fn touch_note(tx: &mut kubuno_db::DbTx, note_id: Uuid) -> Result<u64, sqlx::Error> {
    kubuno_db::journal::touch(tx, NOTES_TABLE, CHANGE_COUNTER, NOTE_DOMAIN, "id", note_id)
        .await
        .map(|(_, affected)| affected)
}

/// Writes a **note** tombstone in the same transaction as its hard delete.
pub async fn record_note_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, NOTE_TOMBSTONES, id, owner_id, seq).await
}

/// Writes a **notebook** tombstone in the same transaction as its hard delete.
pub async fn record_notebook_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, NOTEBOOK_TOMBSTONES, id, owner_id, seq).await
}

/// Writes a **label** tombstone in the same transaction as its hard delete.
pub async fn record_label_tombstone(
    tx: &mut kubuno_db::DbTx,
    id: Uuid,
    owner_id: Uuid,
    seq: i64,
) -> Result<(), sqlx::Error> {
    kubuno_db::journal::record_tombstone(tx, LABEL_TOMBSTONES, id, owner_id, seq).await
}
