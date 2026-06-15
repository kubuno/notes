use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Note {
    pub id:           Uuid,
    pub owner_id:     Uuid,
    pub notebook_id:  Option<Uuid>,
    pub title:        Option<String>,
    // Le contenu vit dans un fichier .kbnot ; peuplé après le SELECT (depuis le
    // fichier pour le détail, depuis `preview` pour la liste).
    #[sqlx(default)]
    pub content:      String,
    #[sqlx(default)]
    pub content_html: Option<String>,
    pub file_id:      Option<Uuid>,
    pub preview:      String,
    pub note_type:    String,
    pub color:        String,
    pub checklist:    serde_json::Value,
    pub drawing_path: Option<String>,
    pub audio_path:   Option<String>,
    pub transcript:   Option<String>,
    pub mentions:     Vec<Uuid>,
    pub mentioned_by: Vec<Uuid>,
    pub is_pinned:    bool,
    pub is_archived:  bool,
    pub is_trashed:   bool,
    pub trashed_at:   Option<DateTime<Utc>>,
    pub word_count:   i32,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteDto {
    pub title:       Option<String>,
    pub content:     Option<String>,
    pub note_type:   Option<String>,
    pub color:       Option<String>,
    pub checklist:   Option<serde_json::Value>,
    pub notebook_id: Option<Uuid>,
    pub is_pinned:   Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteDto {
    pub title:       Option<String>,
    pub content:     Option<String>,
    pub color:       Option<String>,
    pub checklist:   Option<serde_json::Value>,
    pub notebook_id: Option<Uuid>,
    pub is_pinned:   Option<bool>,
    pub is_archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotesQuery {
    pub notebook_id: Option<Uuid>,
    pub label_id:    Option<Uuid>,
    pub note_type:   Option<String>,
    pub pinned:      Option<bool>,
    pub archived:    Option<bool>,
    pub trashed:     Option<bool>,
    pub search:      Option<String>,
    pub limit:       Option<i64>,
    pub offset:      Option<i64>,
}
