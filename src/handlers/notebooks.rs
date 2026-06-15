use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::{NotesError, Result},
    middleware::NotesUser,
    models::{CreateNotebookDto, UpdateNotebookDto},
    services::notebook_service,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
) -> Result<Json<Value>> {
    let notebooks = notebook_service::list_notebooks(&state.db, user.id)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(json!({ "notebooks": notebooks })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Json(dto): Json<CreateNotebookDto>,
) -> Result<(StatusCode, Json<Value>)> {
    if dto.name.trim().is_empty() {
        return Err(NotesError::Validation("Le nom est requis".into()));
    }
    let notebook = notebook_service::create_notebook(&state.db, user.id, dto)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                NotesError::Conflict("Un notebook avec ce nom existe déjà".into())
            } else {
                NotesError::Internal(e)
            }
        })?;
    Ok((StatusCode::CREATED, Json(json!({ "notebook": notebook }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateNotebookDto>,
) -> Result<Json<Value>> {
    let notebook = notebook_service::update_notebook(&state.db, id, user.id, dto)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Notebook {id}")))?;
    Ok(Json(json!({ "notebook": notebook })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let found = notebook_service::delete_notebook(&state.db, id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Notebook {id}"))); }
    Ok(StatusCode::NO_CONTENT)
}
