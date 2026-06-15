//! Stockage du CONTENU des notes dans le module `files` (plus en base).
//!
//! Format Kubuno propre aux notes — MIME `application/vnd.kubuno.note+json`,
//! extension `.kbnot`, JSON gzippé. La base ne garde que la référence `file_id`,
//! un aperçu tronqué (`preview`) pour la liste, et le `search_vector` dérivé
//! (index FTS — le contenu lui-même n'est jamais stocké en base).
//!
//! Les notes vivent dans le dossier **protégé** `Notes/` (icône Lucide StickyNote).

use bytes::Bytes;
use serde_json::{json, Value};
use std::io::{Read as _, Write as _};
use uuid::Uuid;

use crate::{errors::NotesError, state::AppState};

pub const NOTE_MIME: &str = "application/vnd.kubuno.note+json";

// ── Compression (gzip) ──────────────────────────────────────────────────────

fn gzip(raw: &[u8]) -> Result<Vec<u8>, NotesError> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(raw).map_err(|e| NotesError::Internal(anyhow::anyhow!(e)))?;
    enc.finish().map_err(|e| NotesError::Internal(anyhow::anyhow!(e)))
}

fn gunzip(raw: &[u8]) -> Result<Vec<u8>, NotesError> {
    if raw.len() >= 2 && raw[0] == 0x1f && raw[1] == 0x8b {
        let mut dec = flate2::read::GzDecoder::new(raw);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).map_err(|e| NotesError::Internal(anyhow::anyhow!(e)))?;
        Ok(out)
    } else {
        Ok(raw.to_vec())
    }
}

// ── Contenu {content (markdown), content_html} ──────────────────────────────

pub fn note_content_from(content: &str, content_html: &str) -> Value {
    json!({ "version": 1, "content": content, "content_html": content_html })
}

/// Retourne (content, content_html) à partir de l'enveloppe du fichier.
pub fn extract_note(content: &Value) -> (String, String) {
    let c  = content.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let h  = content.get("content_html").and_then(|v| v.as_str()).unwrap_or("").to_string();
    (c, h)
}

fn kb_file_name(title: Option<&str>) -> String {
    let base = title.map(|t| t.trim()).filter(|t| !t.is_empty()).unwrap_or("Note");
    let base = std::path::Path::new(base).file_stem().and_then(|s| s.to_str()).unwrap_or(base);
    format!("{base}.kbnot")
}

pub async fn create_note_file(
    state: &AppState, user_id: Uuid, title: Option<&str>, content: &str, content_html: &str,
) -> Result<Uuid, NotesError> {
    // Dossier protégé Notes/ avec icône.
    let folder = state.files_client.ensure_folder_path(user_id, "Notes", true, Some("StickyNote")).await
        .map_err(NotesError::Internal)?;
    let raw = serde_json::to_vec(&note_content_from(content, content_html))
        .map_err(|e| NotesError::Internal(anyhow::anyhow!(e)))?;
    let gz  = gzip(&raw)?;
    let file = state.files_client.create_file_with_content(
        user_id, Some(folder.id), &kb_file_name(title), NOTE_MIME, Bytes::from(gz),
        Some(json!({ "module": "notes", "subtype": "note" })), false,
    ).await.map_err(NotesError::Internal)?;
    Ok(file.id)
}

pub async fn read_note(state: &AppState, user_id: Uuid, file_id: Uuid) -> Result<(String, String), NotesError> {
    let (_info, raw) = state.files_client.get_file_content(user_id, file_id).await
        .map_err(NotesError::Internal)?;
    let json = gunzip(&raw)?;
    let content = serde_json::from_slice::<Value>(&json)
        .map_err(|e| NotesError::Internal(anyhow::anyhow!("note illisible: {e}")))?;
    Ok(extract_note(&content))
}

pub async fn write_note(state: &AppState, user_id: Uuid, file_id: Uuid, content: &str, content_html: &str) -> Result<(), NotesError> {
    let raw = serde_json::to_vec(&note_content_from(content, content_html))
        .map_err(|e| NotesError::Internal(anyhow::anyhow!(e)))?;
    let gz  = gzip(&raw)?;
    state.files_client.update_file_content(user_id, file_id, Bytes::from(gz)).await
        .map_err(NotesError::Internal).map(|_| ())
}

pub async fn delete_note_file(state: &AppState, user_id: Uuid, file_id: Uuid) {
    let _ = state.files_client.delete_file(user_id, file_id).await;
}

/// Aperçu tronqué (texte brut) pour l'affichage en liste — pas le contenu complet.
pub fn make_preview(content: &str) -> String {
    let flat: String = content.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.chars().take(280).collect()
}


// ── Noms de fichiers : DÉLÉGUÉS à la face client du module `files` ────────────
pub fn strip_ext(name: &str) -> String { crate::files_client::strip_ext(name) }
/// Nom complet du fichier .kb*** (best-effort).
pub async fn file_name(state: &crate::state::AppState, owner_id: uuid::Uuid, file_id: uuid::Uuid) -> Option<String> {
    state.files_client.get_file_meta(owner_id, file_id).await.ok().map(|i| i.name)
}
/// Renomme le fichier .kb*** pour qu'il porte `<title>.<ext>` (titre = nom). Best-effort.
pub async fn rename_content_file(state: &crate::state::AppState, owner_id: uuid::Uuid, file_id: uuid::Uuid, title: &str, ext: &str) {
    crate::files_client::set_title(&state.files_client, owner_id, file_id, title, ext).await
}
