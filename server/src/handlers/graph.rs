use axum::{extract::State, Extension, Json};
use serde_json::Value;

use crate::{
    errors::{NotesError, Result},
    middleware::NotesUser,
    services::backlink_service,
    state::AppState,
};

pub async fn graph(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
) -> Result<Json<Value>> {
    // Bidirectional links disabled instance-wide: an empty graph.
    if !state.instance().enable_bidirectional_links {
        return Ok(Json(serde_json::json!({ "nodes": [], "edges": [] })));
    }
    let data = backlink_service::graph_data(user.id, &state.db)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(serde_json::to_value(data).unwrap()))
}
