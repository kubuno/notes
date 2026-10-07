//! Instance-wide settings of the notes module that the BACKEND acts on, as the
//! administrator left them in the console.
//!
//! Declared by `module.toml`'s `[[settings]]`, stored in `core.settings`, and read
//! back through `/internal/modules/notes/settings`. Only settings applied by the
//! backend live here; the editor-side ones (autosave cadence, spell check) are
//! declared `public` and read by the frontend from `/api/v1/config`.
//!
//! Every field here is read by code that acts on it.

use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct InstanceConfig {
    /// Whether `[[wiki links]]` between notes are resolved and back-linked. Off
    /// stops the backlink graph from being populated.
    pub enable_bidirectional_links: bool,
    /// Whether users may mint link-shared notes at all. Off both refuses new
    /// links and stops serving the ones already minted.
    pub allow_public_sharing: bool,
    /// Ceiling on a share link's lifetime, in days. `0` = no ceiling. A link
    /// asking for longer, or for no expiry at all, is clamped to this many days.
    pub share_link_max_days: i64,
    /// Ceiling, in bytes, on a saved note body. A larger save is rejected.
    pub max_note_size: u64,
    /// Days a trashed note is kept before the cleaner purges it. `0` = never.
    pub trash_retention_days: i32,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            enable_bidirectional_links: true,
            allow_public_sharing:       true,
            share_link_max_days:        0,
            // Same default as `Settings::load` sets for `notes.max_content_size`.
            max_note_size:              1_048_576,
            trash_retention_days:       30,
        }
    }
}

impl InstanceConfig {
    /// Maps the core's `{key: value}` object onto the struct, each read falling
    /// back to the compiled default rather than to a permissive value. An
    /// out-of-range number is treated as a mistake and ignored the same way;
    /// `0` is MEANINGFUL for the two "days" settings (no ceiling / never purge)
    /// and is therefore accepted rather than floored away.
    pub fn from_settings(settings: &Value) -> Self {
        let d = Self::default();
        let int_in = |key: &str, min: i64, max: i64, fallback: i64| -> i64 {
            settings
                .get(key)
                .and_then(Value::as_i64)
                .filter(|n| (min..=max).contains(n))
                .unwrap_or(fallback)
        };
        let bool_of = |key: &str, fallback: bool| {
            settings.get(key).and_then(Value::as_bool).unwrap_or(fallback)
        };
        Self {
            enable_bidirectional_links: bool_of("enable_bidirectional_links", d.enable_bidirectional_links),
            allow_public_sharing:       bool_of("allow_public_sharing", d.allow_public_sharing),
            share_link_max_days:        int_in("share_link_max_days", 0, 3650, d.share_link_max_days),
            max_note_size:              int_in("max_note_size", 1_024, 104_857_600, d.max_note_size as i64) as u64,
            trash_retention_days:       int_in("trash_retention_days", 0, 3650, d.trash_retention_days as i64) as i32,
        }
    }
}

/// Reads the instance settings from the core. Any failure yields `None`, so the
/// caller keeps the values it already had rather than reverting to defaults
/// because the core was briefly unreachable.
pub async fn fetch(http: &reqwest::Client, core_url: &str, secret: &str) -> Option<InstanceConfig> {
    let url = format!("{core_url}/internal/modules/notes/settings");
    let resp = http
        .get(&url)
        .header("X-Internal-Secret", secret)
        .send()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Lecture des réglages d'instance notes"))
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "Réglages d'instance notes refusés par le core");
        return None;
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Réglages d'instance notes : réponse illisible"))
        .ok()?;

    Some(InstanceConfig::from_settings(body.get("settings")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_keys_keep_the_compiled_defaults() {
        let c = InstanceConfig::from_settings(&json!({}));
        assert!(c.enable_bidirectional_links);
        assert!(c.allow_public_sharing);
        assert_eq!(c.share_link_max_days, 0);
        assert_eq!(c.max_note_size, 1_048_576);
        assert_eq!(c.trash_retention_days, 30);
    }

    #[test]
    fn zero_is_meaningful_for_days_settings() {
        let c = InstanceConfig::from_settings(&json!({
            "share_link_max_days": 0, "trash_retention_days": 0,
        }));
        assert_eq!(c.share_link_max_days, 0);  // no ceiling
        assert_eq!(c.trash_retention_days, 0); // never purge
    }

    #[test]
    fn out_of_range_note_size_falls_back() {
        let c = InstanceConfig::from_settings(&json!({ "max_note_size": 12 }));
        assert_eq!(c.max_note_size, 1_048_576);
    }

    #[test]
    fn admin_values_win_over_defaults() {
        let c = InstanceConfig::from_settings(&json!({
            "allow_public_sharing": false, "share_link_max_days": 7, "max_note_size": 4_194_304,
        }));
        assert!(!c.allow_public_sharing);
        assert_eq!(c.share_link_max_days, 7);
        assert_eq!(c.max_note_size, 4_194_304);
    }
}
