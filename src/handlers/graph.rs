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
    let data = backlink_service::graph_data(user.id, &state.db)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(serde_json::to_value(data).unwrap()))
}
