//! Runs the notes module's own migrations and delta/search primitives against a
//! real server of **each** engine, from a single compiled binary — the proof
//! that the engine is a run-time choice, not a build-time one, and that the two
//! recently-ported primitives (the change journal and the normalized full-text
//! search) behave identically on all of them.
//!
//! A note's body is drive-backed (the `.kbnot` files live in the `drive` module,
//! reached over HTTP), so the note CRUD path proper cannot run in a unit test.
//! This binary exercises the DATABASE layer directly, issuing the very SQL the
//! services issue (id + change_seq generated in Rust, `title_norm`/`body_norm`/
//! `transcript_norm` from `search::normalize`), which is what the port changed.
//!
//! * SQLite always runs (a temp file, no server).
//! * PostgreSQL runs when `KUBUNO_PG_TEST_URL` points at a throwaway database.
//! * MySQL/MariaDB runs when `KUBUNO_MYSQL_TEST_URL` does.
//!
//! ```sh
//! KUBUNO_PG_TEST_URL=postgres://u:p@127.0.0.1:5433/notes \
//! KUBUNO_MYSQL_TEST_URL=mysql://u:p@127.0.0.1:3307/notes \
//!   cargo test --test db_portability
//! ```

use kubuno_db::search::{self, Field, Query, Weight};
use kubuno_db::{journal, new_id, params};
use kubuno_notes::{sync, SCHEMA};
use uuid::Uuid;

fn base_settings(engine: &str) -> kubuno_db::DbSettings {
    kubuno_db::DbSettings {
        engine: engine.to_string(),
        url: None,
        host: None,
        port: None,
        user: None,
        password: None,
        database: None,
        path: None,
        max_connections: 4,
        min_connections: 0,
        connect_timeout: std::time::Duration::from_secs(10),
        run_migrations: true,
    }
}

/// Migrations run one at a time: the PostgreSQL and MySQL suites may share a server.
static EXCLUSIVE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn migrated_pool(settings: kubuno_db::DbSettings) -> (kubuno_db::DbPool, impl Sized) {
    let guard = EXCLUSIVE.lock().await;
    let pool = kubuno_db::connect(&settings, SCHEMA).await.expect("connect");
    kubuno_db::migrations!(
        "./migrations/postgres",
        "./migrations/mysql",
        "./migrations/sqlite",
    )
    .run(&pool, SCHEMA)
    .await
    .expect("migrations");
    (pool, guard)
}

// ── Direct DB writes mirroring the services (no drive dependency) ────────────

async fn insert_note(
    pool: &kubuno_db::DbPool,
    owner: Uuid,
    title: &str,
    body: &str,
    transcript: &str,
) -> Uuid {
    let id = new_id();
    let (title_norm, body_norm, transcript_norm) =
        (search::normalize(title), search::normalize(body), search::normalize(transcript));
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_note_seq(&mut tx).await.expect("note seq");
    tx.execute(
        "INSERT INTO notes.notes \
            (id, owner_id, notebook_id, title, note_type, color, checklist, mentions, mentioned_by, \
             is_pinned, file_id, preview, word_count, title_norm, body_norm, transcript_norm, \
             change_seq, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)",
        params![
            id, owner, None::<Uuid>, title, "text", "default", serde_json::json!([]),
            Vec::<Uuid>::new(), Vec::<Uuid>::new(), false, None::<Uuid>, body,
            body.split_whitespace().count() as i32, &title_norm, &body_norm, &transcript_norm,
            seq, chrono::Utc::now(), chrono::Utc::now()
        ],
    )
    .await
    .expect("insert note");
    tx.commit().await.expect("commit");
    id
}

/// Re-edit a note (bumps its change_seq, as `update_note` does).
async fn touch_note(pool: &kubuno_db::DbPool, id: Uuid, new_body: &str) {
    let body_norm = search::normalize(new_body);
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_note_seq(&mut tx).await.expect("note seq");
    tx.execute(
        "UPDATE notes.notes SET preview = $1, body_norm = $2, change_seq = $3 WHERE id = $4",
        params![new_body, &body_norm, seq, id],
    )
    .await
    .expect("update note");
    tx.commit().await.expect("commit");
}

/// Trash a note (a `modified` change carrying is_trashed=true; no tombstone).
async fn trash_note(pool: &kubuno_db::DbPool, id: Uuid) {
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_note_seq(&mut tx).await.expect("note seq");
    tx.execute(
        "UPDATE notes.notes SET is_trashed = $1, change_seq = $2 WHERE id = $3",
        params![true, seq, id],
    )
    .await
    .expect("trash");
    tx.commit().await.expect("commit");
}

/// Hard-delete a note (tombstone + delete, as `delete_note` does).
async fn delete_note(pool: &kubuno_db::DbPool, id: Uuid, owner: Uuid) {
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_note_seq(&mut tx).await.expect("note seq");
    sync::record_note_tombstone(&mut tx, id, owner, seq).await.expect("tombstone");
    tx.execute("DELETE FROM notes.notes WHERE id = $1", params![id]).await.expect("delete");
    tx.commit().await.expect("commit");
}

async fn note_seq(pool: &kubuno_db::DbPool, id: Uuid) -> i64 {
    pool.fetch_scalar::<i64>("SELECT change_seq FROM notes.notes WHERE id = $1", params![id])
        .await
        .expect("note seq")
}

#[derive(Debug, sqlx::FromRow)]
struct TitleRow {
    title: Option<String>,
}

/// The exact search the module runs: `Query::build` over the three normalized,
/// weighted columns, best-ranked first, excluding trashed notes.
async fn search_titles(pool: &kubuno_db::DbPool, owner: Uuid, query: &str) -> Vec<String> {
    let fields = [
        Field::new("title_norm", Weight::A),
        Field::new("body_norm", Weight::B),
        Field::new("transcript_norm", Weight::C),
    ];
    let Some(s) = Query::build(query, &fields, 3) else {
        return Vec::new();
    };
    let sql = format!(
        "SELECT title FROM notes.notes \
         WHERE owner_id = $1 AND is_trashed = $2 AND {} \
         ORDER BY {} DESC, title ASC LIMIT ${}",
        s.where_sql, s.order_sql, s.next
    );
    let mut binds = params![owner, false];
    binds.extend(s.binds);
    binds.push(100i64.into());
    pool.fetch_all_as::<TitleRow>(&sql, binds)
        .await
        .expect("search")
        .into_iter()
        .filter_map(|t| t.title)
        .collect()
}

async fn full_suite(pool: &kubuno_db::DbPool) {
    let owner = Uuid::new_v4();

    // ── notes: strict change_seq monotonicity across create / edit / trash ──
    let mut seqs: Vec<i64> = Vec::new();

    let n1 = insert_note(pool, owner, "Un cheval au galop",
                         "Le cheval broute dans le pré développé", "").await;
    seqs.push(note_seq(pool, n1).await);
    let n2 = insert_note(pool, owner, "Recette de café", "Boire un café le matin", "").await;
    seqs.push(note_seq(pool, n2).await);
    let n3 = insert_note(pool, owner, "Notes de résumé", "Le résumé du projet est prêt", "").await;
    seqs.push(note_seq(pool, n3).await);

    touch_note(pool, n1, "Le cheval broute encore dans le pré").await;
    seqs.push(note_seq(pool, n1).await);

    trash_note(pool, n2).await;
    seqs.push(note_seq(pool, n2).await);

    for w in seqs.windows(2) {
        assert!(w[1] > w[0], "change_seq must strictly increase: {seqs:?}");
    }

    // The full `Note` struct must decode on every engine — checklist (JSON value)
    // and the mentions/mentioned_by JSON arrays are the columns that differ per
    // engine, so this is the load-bearing round-trip the services rely on.
    let full: kubuno_notes::models::Note = pool
        .fetch_one_as::<kubuno_notes::models::Note>(
            "SELECT * FROM notes.notes WHERE id = $1",
            params![n1],
        )
        .await
        .expect("decode full Note");
    assert_eq!(full.title.as_deref(), Some("Un cheval au galop"));
    assert!(full.mentions.is_empty() && full.mentioned_by.is_empty());
    assert_eq!(full.checklist, serde_json::json!([]));

    // ── the owner-scoped feed (journal::changes_since over notes + tombstones) ──
    let feed = journal::changes_since(
        pool, sync::NOTES_TABLE, sync::NOTE_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("notes feed");
    assert!(feed.iter().any(|c| c.id == n1 && !c.deleted), "n1 is a live modified row");
    assert!(feed.iter().any(|c| c.id == n2 && !c.deleted), "trashed n2 stays a live row");
    // The feed is ordered by change_seq.
    let feed_seqs: Vec<i64> = feed.iter().map(|c| c.change_seq).collect();
    let mut sorted = feed_seqs.clone();
    sorted.sort_unstable();
    assert_eq!(feed_seqs, sorted, "the feed is ordered by change_seq");

    // ── search: stemmed + deaccented, identical on every engine ──
    // 1. A plural query word finds the singular stored form (a trashed n2 excluded).
    let hits = search_titles(pool, owner, "chevaux").await;
    assert_eq!(hits, vec!["Un cheval au galop".to_owned()], "chevaux -> cheval");
    // 2. Accent folding: "resume" (no accents) finds "résumé".
    let hits = search_titles(pool, owner, "resume").await;
    assert_eq!(hits, vec!["Notes de résumé".to_owned()], "resume -> résumé");
    // 3. A term absent everywhere returns nothing.
    assert!(search_titles(pool, owner, "hélicoptère").await.is_empty());

    // ── weighting: a title hit (A) outranks a body-only hit (B) ──
    insert_note(pool, owner, "Le lion majestueux", "un grand félin", "").await;
    insert_note(pool, owner, "Félins divers", "le lion et le tigre", "").await;
    let ranked = search_titles(pool, owner, "lion").await;
    assert_eq!(
        ranked,
        vec!["Le lion majestueux".to_owned(), "Félins divers".to_owned()],
        "the title hit ranks above the body-only hit"
    );

    // ── transcript (weight C) is searchable ──
    let voice = insert_note(pool, owner, "Mémo vocal", "", "réunion budgétaire importante").await;
    let hits = search_titles(pool, owner, "budgétaire").await;
    assert!(hits.iter().any(|t| t == "Mémo vocal"), "transcript is searched: {hits:?}");
    let _ = voice;

    // ── tombstone: hard-deleting a note surfaces as a tombstone in the feed ──
    let doomed = insert_note(pool, owner, "Éphémère", "à supprimer", "").await;
    delete_note(pool, doomed, owner).await;
    let feed = journal::changes_since(
        pool, sync::NOTES_TABLE, sync::NOTE_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("notes feed");
    assert!(
        feed.iter().any(|c| c.id == doomed && c.deleted),
        "the deleted note must appear as a tombstone"
    );

    // change_seqs are unique across the whole domain (monotonic counter).
    let mut all_seqs: Vec<i64> = feed.iter().map(|c| c.change_seq).collect();
    let before = all_seqs.len();
    all_seqs.sort_unstable();
    all_seqs.dedup();
    assert_eq!(before, all_seqs.len(), "note change_seqs are unique");

    // ── notebooks + labels: change_seq + tombstone on delete ──
    let nb = insert_notebook(pool, owner, "Travail").await;
    assert!(note_seq_of(pool, "notes.notebooks", nb).await > 0);
    delete_notebook(pool, nb, owner).await;
    let nb_feed = journal::changes_since(
        pool, sync::NOTEBOOKS_TABLE, sync::NOTEBOOK_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("notebooks feed");
    assert!(nb_feed.iter().any(|c| c.id == nb && c.deleted), "deleted notebook tombstoned");

    let lb = insert_label(pool, owner, "urgent").await;
    assert!(note_seq_of(pool, "notes.labels", lb).await > 0);
    delete_label(pool, lb, owner).await;
    let lb_feed = journal::changes_since(
        pool, sync::LABELS_TABLE, sync::LABEL_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("labels feed");
    assert!(lb_feed.iter().any(|c| c.id == lb && c.deleted), "deleted label tombstoned");
}

async fn insert_notebook(pool: &kubuno_db::DbPool, owner: Uuid, name: &str) -> Uuid {
    let id = new_id();
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_notebook_seq(&mut tx).await.expect("notebook seq");
    tx.execute(
        "INSERT INTO notes.notebooks \
            (id, owner_id, parent_id, name, icon, color, position, note_count, change_seq, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,0,0,$7,$8,$9)",
        params![id, owner, None::<Uuid>, name, "📁", None::<&str>, seq, chrono::Utc::now(), chrono::Utc::now()],
    )
    .await
    .expect("insert notebook");
    tx.commit().await.expect("commit");
    id
}

async fn delete_notebook(pool: &kubuno_db::DbPool, id: Uuid, owner: Uuid) {
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_notebook_seq(&mut tx).await.expect("notebook seq");
    sync::record_notebook_tombstone(&mut tx, id, owner, seq).await.expect("tombstone");
    tx.execute("DELETE FROM notes.notebooks WHERE id = $1", params![id]).await.expect("delete");
    tx.commit().await.expect("commit");
}

async fn insert_label(pool: &kubuno_db::DbPool, owner: Uuid, name: &str) -> Uuid {
    let id = new_id();
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_label_seq(&mut tx).await.expect("label seq");
    tx.execute(
        "INSERT INTO notes.labels (id, owner_id, name, color, position, change_seq, created_at) \
         VALUES ($1,$2,$3,$4,0,$5,$6)",
        params![id, owner, name, "#1a73e8", seq, chrono::Utc::now()],
    )
    .await
    .expect("insert label");
    tx.commit().await.expect("commit");
    id
}

async fn delete_label(pool: &kubuno_db::DbPool, id: Uuid, owner: Uuid) {
    let mut tx = pool.begin().await.expect("begin");
    let seq = sync::next_label_seq(&mut tx).await.expect("label seq");
    sync::record_label_tombstone(&mut tx, id, owner, seq).await.expect("tombstone");
    tx.execute("DELETE FROM notes.labels WHERE id = $1", params![id]).await.expect("delete");
    tx.commit().await.expect("commit");
}

async fn note_seq_of(pool: &kubuno_db::DbPool, table: &str, id: Uuid) -> i64 {
    let sql = format!("SELECT change_seq FROM {table} WHERE id = $1");
    pool.fetch_scalar::<i64>(&sql, params![id]).await.expect("seq")
}

#[tokio::test]
async fn sqlite_from_the_one_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = base_settings("sqlite");
    s.path = Some(dir.path().to_string_lossy().into_owned());
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn postgres_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_PG_TEST_URL") else {
        eprintln!("skipping: KUBUNO_PG_TEST_URL not set");
        return;
    };
    let mut s = base_settings("postgres");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn mysql_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_MYSQL_TEST_URL") else {
        eprintln!("skipping: KUBUNO_MYSQL_TEST_URL not set");
        return;
    };
    let mut s = base_settings("mysql");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}
