use anyhow::{Context, Result};
use kubuno_db::{params, DbPool, JsonVec};
use uuid::Uuid;

use crate::services::markdown_service;
use crate::sync;

#[derive(sqlx::FromRow)]
struct IdRow {
    id: Uuid,
}

/// Recomputes this note's `[[wiki links]]` and keeps both sides of every link in
/// step: the note's own `mentions`, and each target's `mentioned_by`.
///
/// The backlink arrays are JSON-array columns (portable across the three
/// engines), so they are read into `Vec<Uuid>`, mutated in Rust and written back
/// whole. Every write is a note change, so it takes a fresh `change_seq` from the
/// journal in the same transaction — the portable replacement for the old
/// `BEFORE UPDATE` seq trigger.
pub async fn update_backlinks(
    note_id: Uuid,
    owner_id: Uuid,
    content: &str,
    db: &DbPool,
) -> Result<()> {
    let mentioned_titles = markdown_service::extract_wikilinks(content);

    let mentioned_ids: Vec<Uuid> = if mentioned_titles.is_empty() {
        vec![]
    } else {
        let lower_titles: Vec<String> =
            mentioned_titles.iter().map(|t| t.to_lowercase()).collect();
        // owner_id = $1, then the title list, then the self-exclusion id.
        let list = db.backend().in_list(2, lower_titles.len());
        let excl = lower_titles.len() + 2;
        let sql = format!(
            "SELECT id FROM notes.notes \
             WHERE owner_id = $1 \
               AND LOWER(COALESCE(title, '')) IN ({list}) \
               AND is_trashed = FALSE \
               AND id <> ${excl}"
        );
        let mut binds = params![owner_id];
        for t in &lower_titles {
            binds.push(t.into());
        }
        binds.push(note_id.into());
        db.fetch_all_as::<IdRow>(&sql, binds)
            .await
            .context("backlinks: resolve titles")?
            .into_iter()
            .map(|r| r.id)
            .collect()
    };

    // Old mentions of this note.
    let old_mentions: Vec<Uuid> = db
        .fetch_optional_as::<MentionsRow>(
            "SELECT mentions FROM notes.notes WHERE id = $1",
            params![note_id],
        )
        .await
        .context("backlinks: read old mentions")?
        .map(|r| r.mentions)
        .unwrap_or_default();

    let mut tx = db.begin().await.context("backlinks begin")?;

    // This note's own mentions (a note change → fresh seq).
    let seq = sync::next_note_seq(&mut tx).await.context("note seq")?;
    tx.execute(
        "UPDATE notes.notes SET mentions = $1, change_seq = $2 WHERE id = $3",
        params![mentioned_ids.clone(), seq, note_id],
    )
    .await
    .context("backlinks: update mentions")?;

    // Remove this note from the mentioned_by of targets it no longer links to.
    let removed: Vec<Uuid> = old_mentions
        .iter()
        .filter(|id| !mentioned_ids.contains(id))
        .copied()
        .collect();
    for target in &removed {
        set_mentioned_by(&mut tx, *target, |mb| mb.retain(|id| *id != note_id)).await?;
    }

    // Add this note to the mentioned_by of newly-linked targets.
    let added: Vec<Uuid> = mentioned_ids
        .iter()
        .filter(|id| !old_mentions.contains(id))
        .copied()
        .collect();
    for target in &added {
        set_mentioned_by(&mut tx, *target, |mb| {
            if !mb.contains(&note_id) {
                mb.push(note_id);
            }
        })
        .await?;
    }

    tx.commit().await.context("backlinks commit")?;
    Ok(())
}

/// Read-modify-writes one target's `mentioned_by` JSON array, bumping its
/// change_seq. A missing target is a no-op (it may have been deleted meanwhile).
async fn set_mentioned_by(
    tx: &mut kubuno_db::DbTx,
    target: Uuid,
    mutate: impl FnOnce(&mut Vec<Uuid>),
) -> Result<()> {
    let row = tx
        .fetch_optional_row(
            "SELECT mentioned_by FROM notes.notes WHERE id = $1",
            params![target],
        )
        .await
        .context("backlinks: read mentioned_by")?;
    let Some(row) = row else { return Ok(()) };
    let mut mb: Vec<Uuid> = row
        .try_get::<JsonVec<Uuid>>("mentioned_by")
        .context("backlinks: decode mentioned_by")?
        .into_inner();
    mutate(&mut mb);
    let seq = sync::next_note_seq(tx).await.context("note seq")?;
    tx.execute(
        "UPDATE notes.notes SET mentioned_by = $1, change_seq = $2 WHERE id = $3",
        params![mb, seq, target],
    )
    .await
    .context("backlinks: write mentioned_by")?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct MentionsRow {
    #[sqlx(json)]
    mentions: Vec<Uuid>,
}

#[derive(serde::Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(serde::Serialize)]
pub struct GraphNode {
    pub id:          String,
    pub label:       String,
    pub note_type:   String,
    pub color:       String,
    pub connections: usize,
    pub word_count:  i32,
}

#[derive(serde::Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to:   String,
}

#[derive(sqlx::FromRow)]
struct GraphRow {
    id:         Uuid,
    title:      Option<String>,
    note_type:  String,
    color:      String,
    #[sqlx(json)]
    mentions:   Vec<Uuid>,
    word_count: i32,
}

pub async fn graph_data(owner_id: Uuid, db: &DbPool) -> Result<GraphData> {
    let rows = db
        .fetch_all_as::<GraphRow>(
            "SELECT id, title, note_type, color, mentions, word_count \
             FROM notes.notes WHERE owner_id = $1 AND is_trashed = $2 AND is_archived = $3",
            params![owner_id, false, false],
        )
        .await
        .context("graph_data")?;

    let mut edges: Vec<GraphEdge> = Vec::new();
    let nodes: Vec<GraphNode> = rows
        .iter()
        .map(|r| {
            let connections = r.mentions.len();
            for target in &r.mentions {
                edges.push(GraphEdge {
                    from: r.id.to_string(),
                    to: target.to_string(),
                });
            }
            GraphNode {
                id: r.id.to_string(),
                label: r.title.clone().unwrap_or_else(|| "Sans titre".into()),
                note_type: r.note_type.clone(),
                color: r.color.clone(),
                connections,
                word_count: r.word_count,
            }
        })
        .collect();

    Ok(GraphData { nodes, edges })
}
