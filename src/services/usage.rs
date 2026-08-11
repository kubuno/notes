//! Declaring to the core what **notes itself** stores, per account.
//!
//! ## The attribution rule, and why notes declares so little
//!
//! Whoever physically holds the byte declares it, and only them.
//!
//! Since migration `000003`, a note's body no longer lives in this database: it
//! is a `.kbnot` file written into **drive** through `FilesClient`, referenced by
//! `notes.notes.file_id`. Drive weighs those bytes and declares them as
//! `content`. Notes declaring them again would be exactly the double counting
//! this channel exists to prevent, so the body is never emitted here — the
//! `delegated` line names the objects instead, with no weight, because notes
//! knows how many it caused and drive knows what they weigh.
//!
//! ## What notes actually holds
//!
//! * `content` — the parts of a note that exist **only** here and that the
//!   account can delete: the checklist items and a voice note's transcript.
//!   `content_files` writes the markdown body to drive; it does not mirror these
//!   columns into the file, so nobody else is counting them. Billed, because the
//!   rule is "an account is billed for what it can free itself" and deleting the
//!   note frees exactly this.
//! * `cache` — the truncated `preview` shown in the list, and the tombstone
//!   journals the offline sync replays. Both are regenerable: the preview from
//!   the drive file, the tombstones from the fact that the row is gone. Not
//!   billed.
//! * `index` — the `search_vector`. The module's own lookup structure, built at
//!   write time from content it does not keep. Not billed.
//! * `delegated` — how many notes caused a drive file to exist. Weight is
//!   deliberately zero and the core never adds this line to any total; it exists
//!   so that "notes looks like it stores nothing" reads as *delegation* rather
//!   than as a broken reporter.
//!
//! ## How bytes are measured
//!
//! `pg_column_size()` throughout, never `octet_length()`. These figures state
//! what the module occupies **in the database**, and PostgreSQL stores this text
//! and JSONB TOASTed and compressed: the logical length would over-state a long
//! checklist by its whole compression ratio. `pg_column_size()` is the figure
//! that matches what the instance actually holds.
//!
//! ## State, never deltas
//!
//! Each declaration carries notes' **current** figures for the accounts and
//! categories it names, so re-sending one changes nothing and a message lost in
//! flight costs one stale number until the next declaration repairs it. The core
//! keys rows on `(module_id, user_id, category)`; idempotence is structural.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::AppState;

/// How often the complete state is recounted and declared. Same period as the
/// other modules, so a console refresh does not show one of them systematically
/// staler than the rest.
const FULL_SYNC_INTERVAL: Duration = Duration::from_secs(6 * 3_600);

/// First retry delay when a declaration could not be delivered, doubling up to
/// [`FULL_SYNC_INTERVAL`].
///
/// The module starts before the core has necessarily finished accepting
/// registrations, so the very first declaration routinely fails. Without a
/// backoff it would be re-attempted six hours later and the breakdown would sit
/// empty for an afternoon after every reboot.
const FULL_RETRY_MIN: Duration = Duration::from_secs(15);

/// Matches the core's own per-request ceiling (`storage::usage::MAX_ENTRIES`).
const MAX_ENTRIES: usize = 5_000;

/// Identifier this module declares under. Only consulted by the core when the
/// caller could not be identified from its `X-Internal-Secret`: the core prefers
/// the secret's identity and answers 403 when the two disagree, so naming
/// ourselves in the body can never impersonate another module.
const MODULE_ID: &str = "notes";

// ── The closed category vocabulary, as notes uses it ─────────────────────────

/// What exists only here and the account can delete. Billed.
const CAT_CONTENT: &str = "content";
/// Regenerable display and sync machinery. Not billed.
const CAT_CACHE: &str = "cache";
/// The module's own full-text lookup structure. Not billed.
const CAT_INDEX: &str = "index";
/// Objects notes made drive create. Never added to any total, by contract.
const CAT_DELEGATED: &str = "delegated";

/// Every byte-bearing query notes runs, paired with the category it feeds.
///
/// Each statement must return exactly `(owner uuid, bytes bigint, objects bigint)`
/// and must only read the `notes` schema. Keeping them in one table rather than
/// scattered through functions is what lets the tests below assert, mechanically,
/// that notes never declares a note's body.
const OWNED_QUERIES: &[(&str, &str)] = &[
    // The columns a note keeps for itself. `title` is deliberately included: it
    // is typed by the person, it is not written into the `.kbnot` body, and
    // leaving it out would mean the one part of a note visible in every list is
    // the one part nobody accounts for.
    (
        CAT_CONTENT,
        "SELECT owner_id,
                COALESCE(SUM(
                    COALESCE(pg_column_size(title), 0)
                  + pg_column_size(checklist)
                  + COALESCE(pg_column_size(transcript), 0)
                ), 0)::bigint,
                COUNT(*)::bigint
           FROM notes.notes
          GROUP BY owner_id",
    ),
    // Notebooks and labels: the shelves, not the books. Created by the account,
    // deleted by the account, held by nobody else.
    (
        CAT_CONTENT,
        "SELECT owner_id,
                COALESCE(SUM(pg_column_size(name)), 0)::bigint,
                COUNT(*)::bigint
           FROM notes.notebooks
          GROUP BY owner_id",
    ),
    (
        CAT_CONTENT,
        "SELECT owner_id,
                COALESCE(SUM(pg_column_size(name)), 0)::bigint,
                COUNT(*)::bigint
           FROM notes.labels
          GROUP BY owner_id",
    ),
    // The list preview: a truncation of the drive file, rebuilt on every write by
    // `content_files::make_preview`. Cache by construction.
    (
        CAT_CACHE,
        "SELECT owner_id,
                COALESCE(SUM(pg_column_size(preview)), 0)::bigint,
                COUNT(*)::bigint
           FROM notes.notes
          GROUP BY owner_id",
    ),
    // Tombstones exist so an offline client learns what disappeared while it was
    // away. They describe deletions, they are pruned by the sync itself, and no
    // account asked for them.
    (
        CAT_CACHE,
        "SELECT owner_id, 0::bigint, COUNT(*)::bigint
           FROM (
                     SELECT owner_id FROM notes.note_tombstones
           UNION ALL SELECT owner_id FROM notes.notebook_tombstones
           UNION ALL SELECT owner_id FROM notes.label_tombstones
           ) t
          GROUP BY owner_id",
    ),
    (
        CAT_INDEX,
        "SELECT owner_id,
                COALESCE(SUM(COALESCE(pg_column_size(search_vector), 0)), 0)::bigint,
                COUNT(*)::bigint
           FROM notes.notes
          GROUP BY owner_id",
    ),
];

/// Counts the drive files notes caused to exist, per owner.
///
/// One row per note holding a non-null `file_id`. The weight is not measured
/// here on purpose: drive holds those bytes and declares them as `content`, and
/// any figure put in this line would be a guess the console might one day add up.
const DELEGATED_QUERY: &str = "SELECT owner_id, COUNT(*)::bigint
       FROM notes.notes
      WHERE file_id IS NOT NULL
      GROUP BY owner_id";

/// One `(account, category)` figure, as declared.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    user_id: Uuid,
    category: &'static str,
    used_bytes: i64,
    object_count: i64,
}

/// Recounts everything notes holds, plus the delegation line.
///
/// A failing query is logged and skipped rather than aborting the whole
/// declaration: one broken table should cost its own line, not the entire
/// breakdown. The declaration is still marked `full`, which means a category that
/// failed here is *retired* by the core until the next sync repairs it — the
/// honest outcome, since publishing a stale figure as current state would be
/// worse than publishing none.
async fn collect(db: &PgPool) -> Vec<Entry> {
    // Several queries feed the same category (notes, notebooks and labels all
    // feed `content`), so figures are folded per `(user, category)` before being
    // sent: the core keys rows on that pair and would keep only the last one.
    let mut acc: HashMap<(Uuid, &'static str), (i64, i64)> = HashMap::new();

    for (category, sql) in OWNED_QUERIES {
        match sqlx::query_as::<_, (Uuid, i64, i64)>(sql).fetch_all(db).await {
            Ok(rows) => {
                for (user_id, bytes, objects) in rows {
                    let slot = acc.entry((user_id, *category)).or_insert((0, 0));
                    slot.0 += bytes;
                    slot.1 += objects;
                }
            }
            Err(e) => tracing::error!(
                error = %e,
                catégorie = *category,
                "Recomptage de consommation échoué pour une requête — catégorie incomplète"
            ),
        }
    }

    match sqlx::query_as::<_, (Uuid, i64)>(DELEGATED_QUERY)
        .fetch_all(db)
        .await
    {
        Ok(rows) => {
            for (user_id, objects) in rows {
                // `used_bytes: 0` is deliberate and load-bearing. Notes knows how
                // many files it caused, never what they weigh — drive weighs them
                // and declares them as `content`.
                let slot = acc.entry((user_id, CAT_DELEGATED)).or_insert((0, 0));
                slot.1 += objects;
            }
        }
        Err(e) => tracing::error!(error = %e, "Recomptage des objets délégués à drive échoué"),
    }

    let mut entries: Vec<Entry> = acc
        .into_iter()
        .map(|((user_id, category), (used_bytes, object_count))| Entry {
            user_id,
            category,
            used_bytes,
            object_count,
        })
        .collect();

    // Stable order so consecutive declarations chunk identically — a moving
    // chunk boundary would make partial declarations retire different accounts
    // each time.
    entries.sort_by(|a, b| (a.user_id, a.category).cmp(&(b.user_id, b.category)));
    entries
}

/// How many calls a declaration of `n` entries takes. Zero entries still takes
/// one: an empty `full` declaration is a statement, not a no-op.
fn page_count(n: usize) -> usize {
    if n == 0 { 1 } else { n.div_ceil(MAX_ENTRIES) }
}

/// Whether `full` may actually be claimed on the wire.
///
/// A chunked declaration marked `full` would retire every entry outside whichever
/// chunk happened to be sent last. Beyond the ceiling it is therefore sent as
/// partial: correct, but unable to retire a line until the instance drops back
/// under `MAX_ENTRIES`.
fn claims_full(full: bool, n: usize) -> bool {
    full && n <= MAX_ENTRIES
}

/// Sends one declaration to the core.
async fn send(
    http: &reqwest::Client,
    state: &AppState,
    entries: &[Entry],
    full: bool,
) -> Result<(), String> {
    let url = format!("{}/internal/storage/usage", state.settings.core.url);
    let usage: Vec<_> = entries
        .iter()
        .map(|e| {
            json!({
                "user_id":      e.user_id,
                "category":     e.category,
                "used_bytes":   e.used_bytes,
                "object_count": e.object_count,
            })
        })
        .collect();

    let resp = http
        .post(&url)
        .header(
            "X-Internal-Secret",
            state.settings.core.internal_secret.as_str(),
        )
        .json(&json!({ "module_id": MODULE_ID, "full": full, "usage": usage }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        return Ok(());
    }

    // The status alone does not say which of several validations refused the
    // declaration, and this runs unattended — a log line reading "HTTP 422" costs
    // an afternoon the next time the contract shifts.
    let status = resp.status();
    let detail = resp.text().await.unwrap_or_default();
    let detail: String = detail.chars().take(300).collect();
    Err(format!("HTTP {status} {detail}"))
}

/// Declares `entries` in as many calls as the core's ceiling requires.
///
/// Returns `true` when every call landed; the caller reschedules on that.
async fn declare(http: &reqwest::Client, state: &AppState, entries: Vec<Entry>, full: bool) -> bool {
    let full_on_wire = claims_full(full, entries.len());
    if full && !full_on_wire {
        tracing::warn!(
            entrées = entries.len(),
            envois = page_count(entries.len()),
            "Synchronisation complète découpée : déclarée en plusieurs envois partiels"
        );
    }

    // An empty full declaration is meaningful and must still be sent: it is how
    // notes says "I hold nothing for anybody", which the core has to be able to
    // tell apart from "notes has never declared".
    if entries.is_empty() {
        if !full_on_wire {
            return true;
        }
        return match send(http, state, &[], true).await {
            Ok(()) => {
                tracing::debug!("Consommation déclarée : aucune entrée");
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "Déclaration de consommation échouée");
                false
            }
        };
    }

    let mut declared_bytes: i64 = 0;
    let mut sent = 0usize;
    let mut all_ok = true;
    for chunk in entries.chunks(MAX_ENTRIES) {
        match send(http, state, chunk, full_on_wire).await {
            Ok(()) => {
                declared_bytes += chunk.iter().map(|e| e.used_bytes).sum::<i64>();
                sent += chunk.len();
            }
            Err(e) => {
                all_ok = false;
                tracing::warn!(error = %e, entrées = chunk.len(), "Déclaration de consommation échouée");
            }
        }
    }

    if sent > 0 {
        tracing::debug!(
            entrées = sent,
            octets = declared_bytes,
            complète = full_on_wire,
            "Consommation déclarée au core"
        );
    }
    all_ok
}

/// The reporter task. Started once at bootstrap.
pub async fn run_reporter(state: AppState) {
    let http = reqwest::Client::new();

    // Absolute deadline rather than an `interval`, so a failed sync can be pulled
    // forward without the retries drifting the normal period.
    let mut next_at = tokio::time::Instant::now(); // the first one is immediate
    let mut backoff = FULL_RETRY_MIN;

    tracing::info!("Rapporteur de consommation démarré (déclaration au core)");

    loop {
        tokio::time::sleep_until(next_at).await;

        let entries = collect(&state.db).await;
        let delivered = declare(&http, &state, entries, true).await;

        let now = tokio::time::Instant::now();
        if delivered {
            next_at = now + FULL_SYNC_INTERVAL;
            backoff = FULL_RETRY_MIN;
        } else {
            next_at = now + backoff;
            backoff = (backoff * 2).min(FULL_SYNC_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule this module exists to respect, checked mechanically.
    ///
    /// A note's body is a drive file. Notes measuring it — by reading `file_id`
    /// for anything other than counting, or by weighing a column that mirrors the
    /// body — would double-count exactly the quantity the core bills.
    #[test]
    fn the_body_is_never_weighed_here() {
        for (category, sql) in OWNED_QUERIES {
            assert!(
                !sql.to_lowercase().contains("file_id"),
                "une requête pesée touche à file_id — le corps appartient à drive : {sql}"
            );
            assert_ne!(
                *category, CAT_DELEGATED,
                "la délégation ne passe pas par OWNED_QUERIES : elle ne porte aucun poids"
            );
        }
        assert!(
            DELEGATED_QUERY.to_lowercase().contains("count(*)"),
            "la ligne déléguée compte des objets, elle ne pèse rien"
        );
        for weighed in ["pg_column_size", "sum("] {
            assert!(
                !DELEGATED_QUERY.to_lowercase().contains(weighed),
                "la ligne déléguée pèserait des octets que drive déclare déjà"
            );
        }
    }

    /// Only the four categories notes genuinely uses, and `content` never used
    /// for something the account cannot delete.
    #[test]
    fn emitted_categories_are_the_expected_set() {
        use std::collections::BTreeSet;
        let cats: BTreeSet<&str> = OWNED_QUERIES.iter().map(|(c, _)| *c).collect();
        assert_eq!(cats, BTreeSet::from([CAT_CONTENT, CAT_CACHE, CAT_INDEX]));
    }

    /// A module reading another module's schema would be both an architecture
    /// violation and a double count waiting to happen.
    #[test]
    fn queries_only_read_the_notes_schema() {
        for (_, sql) in OWNED_QUERIES.iter().map(|(c, s)| (c, *s)).chain([(&"", DELEGATED_QUERY)]) {
            let lowered = sql.to_lowercase();
            for foreign in ["drive.", "core.", "office.", "chat.", "photos."] {
                assert!(
                    !lowered.contains(foreign),
                    "la requête lit le schéma « {foreign} » : {sql}"
                );
            }
            assert!(
                lowered.contains("notes."),
                "la requête ne lit aucune table de notes : {sql}"
            );
        }
    }

    /// Every statement must yield a line per account.
    #[test]
    fn queries_group_by_an_owner() {
        for (_, sql) in OWNED_QUERIES {
            assert!(
                sql.to_lowercase().contains("group by owner_id"),
                "requête sans GROUP BY owner_id — une ligne par compte est le contrat : {sql}"
            );
        }
        assert!(DELEGATED_QUERY.to_lowercase().contains("group by owner_id"));
    }

    #[test]
    fn paging_respects_the_core_ceiling() {
        assert_eq!(page_count(0), 1, "une déclaration vide reste une déclaration");
        assert_eq!(page_count(MAX_ENTRIES), 1);
        assert_eq!(page_count(MAX_ENTRIES + 1), 2);
    }

    #[test]
    fn full_is_only_claimed_when_it_fits_in_one_call() {
        assert!(claims_full(true, MAX_ENTRIES));
        assert!(
            !claims_full(true, MAX_ENTRIES + 1),
            "une déclaration découpée ne peut pas se dire complète"
        );
        assert!(!claims_full(false, 1));
    }

    #[test]
    fn chunking_covers_every_entry_exactly_once() {
        let entries: Vec<Entry> = (0..MAX_ENTRIES + 3)
            .map(|i| Entry {
                user_id: Uuid::from_u128(i as u128),
                category: CAT_CONTENT,
                used_bytes: 1,
                object_count: 1,
            })
            .collect();
        let seen: usize = entries.chunks(MAX_ENTRIES).map(<[Entry]>::len).sum();
        assert_eq!(seen, entries.len());
    }
}
