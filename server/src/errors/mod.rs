use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum NotesError {
    #[error("Non authentifié")]
    Unauthorized,

    #[error("Accès refusé")]
    Forbidden,

    #[error("Ressource introuvable: {0}")]
    NotFound(String),

    #[error("Données invalides: {0}")]
    Validation(String),

    #[error("Conflit: {0}")]
    Conflict(String),

    #[error("Contenu trop volumineux")]
    ContentTooLarge,

    #[error("Erreur base de données")]
    Database(#[from] sqlx::Error),

    #[error("Erreur interne")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for NotesError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            NotesError::Unauthorized    => (StatusCode::UNAUTHORIZED,           "UNAUTHORIZED",      self.to_string()),
            NotesError::Forbidden       => (StatusCode::FORBIDDEN,              "FORBIDDEN",         self.to_string()),
            NotesError::NotFound(_)     => (StatusCode::NOT_FOUND,              "NOT_FOUND",         self.to_string()),
            NotesError::Validation(_)   => (StatusCode::UNPROCESSABLE_ENTITY,   "VALIDATION",        self.to_string()),
            NotesError::Conflict(_)     => (StatusCode::CONFLICT,               "CONFLICT",          self.to_string()),
            NotesError::ContentTooLarge => (StatusCode::PAYLOAD_TOO_LARGE,      "CONTENT_TOO_LARGE", self.to_string()),
            NotesError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "Erreur base de données".to_string())
            }
            NotesError::Internal(e) => {
                tracing::error!(error = %e, "Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Erreur interne".to_string())
            }
        };

        (status, Json(json!({ "error": code, "message": message }))).into_response()
    }
}

pub type Result<T> = std::result::Result<T, NotesError>;
