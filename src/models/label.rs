use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Label {
    pub id:         Uuid,
    pub owner_id:   Uuid,
    pub name:       String,
    pub color:      String,
    pub position:   i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLabelDto {
    pub name:     String,
    pub color:    Option<String>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLabelDto {
    pub name:     Option<String>,
    pub color:    Option<String>,
    pub position: Option<i32>,
}
