use anyhow::{Context, Result};
use kubuno_db::{new_id, params, DbPool, DbTx};
use uuid::Uuid;

use crate::models::{CreateNotebookDto, Notebook, UpdateNotebookDto};
use crate::sync;

#[derive(sqlx::FromRow)]
struct IdRow {
    id: Uuid,
}

pub async fn list_notebooks(db: &DbPool, owner_id: Uuid) -> Result<Vec<Notebook>> {
    let notebooks = db
        .fetch_all_as::<Notebook>(
            "SELECT * FROM notes.notebooks WHERE owner_id = $1 ORDER BY position, name",
            params![owner_id],
        )
        .await
        .context("list_notebooks")?;
    Ok(notebooks)
}

pub async fn create_notebook(
    db: &DbPool,
    owner_id: Uuid,
    dto: CreateNotebookDto,
) -> Result<Notebook> {
    let id = dto.id.unwrap_or_else(new_id);
    let icon = dto.icon.as_deref().unwrap_or("📁");
    let now = chrono::Utc::now();

    let mut tx = db.begin().await.context("create_notebook begin")?;
    let seq = sync::next_notebook_seq(&mut tx).await.context("notebook seq")?;
    tx.execute(
        "INSERT INTO notes.notebooks \
            (id, owner_id, parent_id, name, icon, color, position, note_count, change_seq, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, 0, 0, $7, $8, $9)",
        params![id, owner_id, dto.parent_id, &dto.name, icon, dto.color.as_deref(), seq, now, now],
    )
    .await
    .context("create_notebook insert")?;
    tx.commit().await.context("create_notebook commit")?;

    reselect_notebook(db, id, owner_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("notebook disparu"))
}

pub async fn update_notebook(
    db: &DbPool,
    id: Uuid,
    owner_id: Uuid,
    dto: UpdateNotebookDto,
) -> Result<Option<Notebook>> {
    let now = chrono::Utc::now();
    let mut tx = db.begin().await.context("update_notebook begin")?;
    let seq = sync::next_notebook_seq(&mut tx).await.context("notebook seq")?;
    let affected = tx
        .execute(
            "UPDATE notes.notebooks \
             SET name       = COALESCE($1, name), \
                 icon       = COALESCE($2, icon), \
                 color      = COALESCE($3, color), \
                 position   = COALESCE($4, position), \
                 change_seq = $5, \
                 updated_at = $6 \
             WHERE id = $7 AND owner_id = $8",
            params![
                dto.name.as_deref(),
                dto.icon.as_deref(),
                dto.color.as_deref(),
                dto.position,
                seq,
                now,
                id,
                owner_id
            ],
        )
        .await
        .context("update_notebook")?;
    tx.commit().await.context("update_notebook commit")?;

    if affected == 0 {
        return Ok(None);
    }
    reselect_notebook(db, id, owner_id).await
}

pub async fn delete_notebook(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<bool> {
    // Confirm ownership first; nothing to do for a stranger's or missing notebook.
    let owns: Option<Uuid> = db
        .fetch_optional_scalar::<Uuid>(
            "SELECT id FROM notes.notebooks WHERE id = $1 AND owner_id = $2",
            params![id, owner_id],
        )
        .await
        .context("delete_notebook check")?;
    if owns.is_none() {
        return Ok(false);
    }

    // The FK cascades to child notebooks and NULLs the notebook_id of notes; in
    // the trigger design those cascade writes each bumped a change_seq. Reproduce
    // it in Rust: collect the whole subtree and its notes up front (through the
    // pool), then in one transaction un-file each note with a fresh seq,
    // tombstone every notebook in the subtree, and delete the root (whose cascade
    // removes the descendant rows, already tombstoned).
    let subtree = descendant_notebook_ids(db, id).await?;
    let note_ids = notes_in_notebooks(db, &subtree).await?;

    let mut tx = db.begin().await.context("delete_notebook begin")?;
    for nid in &note_ids {
        let nseq = sync::next_note_seq(&mut tx).await.context("note seq")?;
        tx.execute(
            "UPDATE notes.notes SET notebook_id = NULL, change_seq = $1 WHERE id = $2",
            params![nseq, nid],
        )
        .await
        .context("delete_notebook unfile note")?;
    }
    for nb in &subtree {
        let bseq = sync::next_notebook_seq(&mut tx).await.context("notebook seq")?;
        sync::record_notebook_tombstone(&mut tx, *nb, owner_id, bseq)
            .await
            .context("delete_notebook tombstone")?;
    }
    tx.execute("DELETE FROM notes.notebooks WHERE id = $1", params![id])
        .await
        .context("delete_notebook")?;
    tx.commit().await.context("delete_notebook commit")?;
    Ok(true)
}

/// Adjusts a notebook's `note_count` by `delta` (a portable replacement for the
/// old `notes_notebook_count` trigger), clamped at 0, and bumps its change_seq
/// so the count change reaches the delta feed. Runs on the caller's transaction.
pub async fn adjust_note_count(tx: &mut DbTx, notebook_id: Uuid, delta: i64) -> Result<(), sqlx::Error> {
    let seq = sync::next_notebook_seq(tx).await?;
    // CASE keeps the count non-negative on every engine (GREATEST/MAX are spelled
    // differently across the three).
    tx.execute(
        "UPDATE notes.notebooks \
         SET note_count = CASE WHEN note_count + $1 < 0 THEN 0 ELSE note_count + $1 END, \
             change_seq = $2 \
         WHERE id = $3",
        params![delta, seq, notebook_id],
    )
    .await?;
    Ok(())
}

/// The full set of notebook ids rooted at `root` (root included), gathered
/// breadth-first over `parent_id`. Depth is tiny, so a BFS avoids the dialect
/// edge cases of `WITH RECURSIVE`.
async fn descendant_notebook_ids(db: &DbPool, root: Uuid) -> Result<Vec<Uuid>> {
    let mut all = vec![root];
    let mut frontier = vec![root];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for parent in frontier {
            let children: Vec<Uuid> = db
                .fetch_all_as::<IdRow>(
                    "SELECT id FROM notes.notebooks WHERE parent_id = $1",
                    params![parent],
                )
                .await
                .context("descendant notebooks")?
                .into_iter()
                .map(|r| r.id)
                .collect();
            next.extend(children);
        }
        all.extend(next.iter().copied());
        frontier = next;
    }
    Ok(all)
}

/// Every note id whose `notebook_id` is one of `notebook_ids`.
async fn notes_in_notebooks(db: &DbPool, notebook_ids: &[Uuid]) -> Result<Vec<Uuid>> {
    if notebook_ids.is_empty() {
        return Ok(Vec::new());
    }
    let list = db.backend().in_list(1, notebook_ids.len());
    let sql = format!("SELECT id FROM notes.notes WHERE notebook_id IN ({list})");
    let mut binds = params![];
    for nb in notebook_ids {
        binds.push((*nb).into());
    }
    let ids = db
        .fetch_all_as::<IdRow>(&sql, binds)
        .await
        .context("notes_in_notebooks")?
        .into_iter()
        .map(|r| r.id)
        .collect();
    Ok(ids)
}

async fn reselect_notebook(db: &DbPool, id: Uuid, owner_id: Uuid) -> Result<Option<Notebook>> {
    db.fetch_optional_as::<Notebook>(
        "SELECT * FROM notes.notebooks WHERE id = $1 AND owner_id = $2",
        params![id, owner_id],
    )
    .await
    .context("reselect_notebook")
}
