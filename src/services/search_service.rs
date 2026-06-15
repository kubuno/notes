use anyhow::{Context, Result};
use uuid::Uuid;

use crate::models::Note;
use crate::state::AppState;

pub async fn search(state: &AppState, owner_id: Uuid, query: &str, limit: i64) -> Result<Vec<Note>> {
    let mut notes = sqlx::query_as::<_, Note>(
        r#"SELECT * FROM notes
           WHERE owner_id = $1
             AND is_trashed = FALSE
             AND search_vector @@ plainto_tsquery('french', $2)
           ORDER BY ts_rank(search_vector, plainto_tsquery('french', $2)) DESC
           LIMIT $3"#,
    )
    .bind(owner_id)
    .bind(query)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .context("search")?;
    // Aperçu pour l'affichage (le contenu complet vit dans le fichier .kbnot).
    for n in &mut notes {
        n.content = n.preview.clone();
    }
    Ok(notes)
}
