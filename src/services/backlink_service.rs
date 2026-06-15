use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::markdown_service;

pub async fn update_backlinks(
    note_id:  Uuid,
    owner_id: Uuid,
    content:  &str,
    db:       &PgPool,
) -> Result<()> {
    let mentioned_titles = markdown_service::extract_wikilinks(content);

    let mentioned_ids: Vec<Uuid> = if mentioned_titles.is_empty() {
        vec![]
    } else {
        let lower_titles: Vec<String> = mentioned_titles.iter().map(|t| t.to_lowercase()).collect();
        sqlx::query_scalar::<_, Uuid>(
            r#"SELECT id FROM notes
               WHERE owner_id = $1
                 AND LOWER(COALESCE(title, '')) = ANY($2)
                 AND is_trashed = FALSE
                 AND id <> $3"#,
        )
        .bind(owner_id)
        .bind(&lower_titles)
        .bind(note_id)
        .fetch_all(db)
        .await
        .context("backlinks: resolve titles")?
    };

    // Récupérer les anciennes mentions
    let old_mentions: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(
        "SELECT unnest(mentions) FROM notes WHERE id = $1",
    )
    .bind(note_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Mettre à jour mentions de cette note
    sqlx::query(
        "UPDATE notes SET mentions = $1 WHERE id = $2",
    )
    .bind(&mentioned_ids)
    .bind(note_id)
    .execute(db)
    .await
    .context("backlinks: update mentions")?;

    // Retirer cette note des mentioned_by des anciennes cibles
    let removed: Vec<Uuid> = old_mentions.iter()
        .filter(|id| !mentioned_ids.contains(id))
        .copied()
        .collect();
    if !removed.is_empty() {
        sqlx::query(
            "UPDATE notes SET mentioned_by = array_remove(mentioned_by, $1) WHERE id = ANY($2)",
        )
        .bind(note_id)
        .bind(&removed)
        .execute(db)
        .await
        .context("backlinks: remove old mentioned_by")?;
    }

    // Ajouter cette note dans mentioned_by des nouvelles cibles
    let added: Vec<Uuid> = mentioned_ids.iter()
        .filter(|id| !old_mentions.contains(id))
        .copied()
        .collect();
    if !added.is_empty() {
        sqlx::query(
            "UPDATE notes SET mentioned_by = array_append(mentioned_by, $1) WHERE id = ANY($2) AND NOT ($1 = ANY(mentioned_by))",
        )
        .bind(note_id)
        .bind(&added)
        .execute(db)
        .await
        .context("backlinks: add new mentioned_by")?;
    }

    Ok(())
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

struct GraphRow {
    id:        Uuid,
    title:     String,
    note_type: String,
    color:     String,
    mentions:  Vec<Uuid>,
    word_count: i32,
}

impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for GraphRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id:         row.try_get("id")?,
            title:      row.try_get::<Option<String>, _>("title")?.unwrap_or_else(|| "Sans titre".into()),
            note_type:  row.try_get("note_type")?,
            color:      row.try_get("color")?,
            mentions:   row.try_get::<Option<Vec<Uuid>>, _>("mentions")?.unwrap_or_default(),
            word_count: row.try_get("word_count")?,
        })
    }
}

pub async fn graph_data(owner_id: Uuid, db: &PgPool) -> Result<GraphData> {
    let rows = sqlx::query_as::<_, GraphRow>(
        "SELECT id, title, note_type, color, mentions, word_count
         FROM notes WHERE owner_id = $1 AND is_trashed = FALSE AND is_archived = FALSE",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
    .context("graph_data")?;

    let mut edges: Vec<GraphEdge> = Vec::new();
    let nodes: Vec<GraphNode> = rows.iter().map(|r| {
        let connections = r.mentions.len();
        for target in &r.mentions {
            edges.push(GraphEdge {
                from: r.id.to_string(),
                to:   target.to_string(),
            });
        }
        GraphNode {
            id:          r.id.to_string(),
            label:       r.title.clone(),
            note_type:   r.note_type.clone(),
            color:       r.color.clone(),
            connections,
            word_count:  r.word_count,
        }
    }).collect();

    Ok(GraphData { nodes, edges })
}
