//! Full-text note search made identical on the three engines by
//! `kubuno_db::search`: a note's title, body (the `.kbnot` content, captured at
//! write time) and a voice note's transcript are reduced to Snowball French
//! stems and deaccented IN RUST at write time (stored in `title_norm` /
//! `body_norm` / `transcript_norm`), and a query is put through the same
//! reduction and matched with a portable `LIKE`. This replaces the old
//! PostgreSQL-only `tsvector` / `ts_rank` / `plainto_tsquery` / `unaccent`
//! pipeline.
//!
//! Ranking mirrors the former `setweight A/B/C`: a hit in the title (weight A)
//! outranks a hit in the body (weight B), which outranks the transcript (C).
//!
//! Reservation: `pg_trgm`'s typo tolerance is gone — a `LIKE '%stem%'` needs the
//! stem to appear as a substring. Stemming still folds inflections and the
//! normalizer folds accents, so inflected and accented queries still match.

use anyhow::{Context, Result};
use kubuno_db::params;
use kubuno_db::search::{Field, Query, Weight};
use uuid::Uuid;

use crate::models::Note;
use crate::state::AppState;

/// The three weighted normalized columns of a note (title > body > transcript).
pub fn fields() -> [Field; 3] {
    [
        Field::new("title_norm", Weight::A),
        Field::new("body_norm", Weight::B),
        Field::new("transcript_norm", Weight::C),
    ]
}

pub async fn search(state: &AppState, owner_id: Uuid, query: &str, limit: i64) -> Result<Vec<Note>> {
    let limit = limit.clamp(1, 200);
    // Pre-conditions: $1 = owner_id, $2 = is_trashed(false). The search filter
    // and score placeholders follow from $3.
    let Some(s) = Query::build(query, &fields(), 3) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "SELECT * FROM notes.notes \
         WHERE owner_id = $1 AND is_trashed = $2 AND {where_sql} \
         ORDER BY {order_sql} DESC, updated_at DESC \
         LIMIT ${limit_ph}",
        where_sql = s.where_sql,
        order_sql = s.order_sql,
        limit_ph = s.next,
    );
    let mut binds = params![owner_id, false];
    binds.extend(s.binds);
    binds.push(limit.into());

    let mut notes: Vec<Note> = state
        .db
        .fetch_all_as::<Note>(&sql, binds)
        .await
        .context("search")?;
    // Aperçu pour l'affichage (le contenu complet vit dans le fichier .kbnot).
    for n in &mut notes {
        n.content = n.preview.clone();
    }
    Ok(notes)
}
