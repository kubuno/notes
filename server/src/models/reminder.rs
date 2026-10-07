use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Reminder {
    pub id:         Uuid,
    pub note_id:    Uuid,
    pub owner_id:   Uuid,
    pub fire_at:    DateTime<Utc>,
    pub method:     String,
    pub recurrence: String,
    pub sent_at:    Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReminderDto {
    pub fire_at:    DateTime<Utc>,
    pub method:     Option<String>,
    pub recurrence: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateReminderDto {
    pub fire_at:    Option<DateTime<Utc>>,
    pub method:     Option<String>,
    pub recurrence: Option<String>,
}
