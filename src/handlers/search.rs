use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    errors::{NotesError, Result},
    middleware::NotesUser,
    services::search_service,
    state::AppState,
};

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q:     String,
    pub limit: Option<i64>,
}

pub async fn search(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>> {
    if q.q.trim().is_empty() {
        return Ok(Json(json!({ "notes": [] })));
    }
    let limit = q.limit.unwrap_or(50).min(200);
    let notes = search_service::search(&state, user.id, &q.q, limit)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(json!({ "notes": notes })))
}
