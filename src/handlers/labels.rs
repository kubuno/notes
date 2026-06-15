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
    models::{CreateLabelDto, UpdateLabelDto},
    services::label_service,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
) -> Result<Json<Value>> {
    let labels = label_service::list_labels(&state.db, user.id)
        .await
        .map_err(NotesError::Internal)?;
    Ok(Json(json!({ "labels": labels })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Json(dto): Json<CreateLabelDto>,
) -> Result<(StatusCode, Json<Value>)> {
    if dto.name.trim().is_empty() {
        return Err(NotesError::Validation("Le nom est requis".into()));
    }
    let label = label_service::create_label(&state.db, user.id, dto)
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") {
                NotesError::Conflict("Un label avec ce nom existe déjà".into())
            } else {
                NotesError::Internal(e)
            }
        })?;
    Ok((StatusCode::CREATED, Json(json!({ "label": label }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateLabelDto>,
) -> Result<Json<Value>> {
    let label = label_service::update_label(&state.db, id, user.id, dto)
        .await
        .map_err(NotesError::Internal)?
        .ok_or_else(|| NotesError::NotFound(format!("Label {id}")))?;
    Ok(Json(json!({ "label": label })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    let found = label_service::delete_label(&state.db, id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Label {id}"))); }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn assign(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path((note_id, label_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    let found = label_service::assign_label(&state.db, note_id, label_id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    if !found { return Err(NotesError::NotFound(format!("Note {note_id}"))); }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove(
    State(state): State<AppState>,
    Extension(user): Extension<NotesUser>,
    Path((note_id, label_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    label_service::remove_label(&state.db, note_id, label_id, user.id)
        .await
        .map_err(NotesError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}
