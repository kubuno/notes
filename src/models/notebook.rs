use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Notebook {
    pub id:         Uuid,
    pub owner_id:   Uuid,
    pub parent_id:  Option<Uuid>,
    pub name:       String,
    pub icon:       String,
    pub color:      Option<String>,
    pub position:   i32,
    pub note_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNotebookDto {
    pub name:      String,
    pub parent_id: Option<Uuid>,
    pub icon:      Option<String>,
    pub color:     Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotebookDto {
    pub name:      Option<String>,
    pub parent_id: Option<Uuid>,
    pub icon:      Option<String>,
    pub color:     Option<String>,
    pub position:  Option<i32>,
}
