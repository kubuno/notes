use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

pub async fn publish_event(
    client: &Client,
    core_url: &str,
    internal_secret: &str,
    event: Value,
) -> Result<()> {
    let url = format!("{core_url}/internal/events/publish");
    client
        .post(&url)
        .header("X-Internal-Secret", internal_secret)
        .json(&event)
        .send()
        .await?;
    Ok(())
}

pub fn note_created_event(note_id: uuid::Uuid, user_id: uuid::Uuid) -> Value {
    json!({
        "type": "NoteCreated",
        "payload": {
            "note_id":   note_id,
            "user_id":   user_id,
            "module_id": "notes"
        }
    })
}
