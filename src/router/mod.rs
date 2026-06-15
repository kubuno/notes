use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{graph, health, labels, notebooks, notes, public, reminders, search, shares},
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Notes
        .route("/notes",                        get(notes::list).post(notes::create))
        .route("/notes/open-by-file",           post(notes::open_by_file))
        .route("/notes/trash",                  delete(notes::empty_trash))
        .route("/notes/:id",                    get(notes::get).patch(notes::update))
        .route("/notes/:id/trash",              post(notes::trash))
        .route("/notes/:id/restore",            post(notes::restore))
        .route("/notes/:id/delete",             delete(notes::delete_permanent))
        .route("/notes/:id/duplicate",          post(notes::duplicate))
        .route("/notes/:id/backlinks",          get(notes::backlinks))
        // Labels
        .route("/labels",                       get(labels::list).post(labels::create))
        .route("/labels/:id",                   patch(labels::update).delete(labels::delete))
        .route("/notes/:note_id/labels/:label_id", post(labels::assign).delete(labels::remove))
        // Notebooks
        .route("/notebooks",                    get(notebooks::list).post(notebooks::create))
        .route("/notebooks/:id",                patch(notebooks::update).delete(notebooks::delete))
        // Reminders
        .route("/notes/:note_id/reminders",     get(reminders::list).post(reminders::create))
        .route("/notes/:note_id/reminders/:id", patch(reminders::update).delete(reminders::delete))
        // Partages
        .route("/notes/:note_id/shares",        get(shares::list).post(shares::create))
        .route("/notes/:note_id/shares/:id",    delete(shares::delete))
        // Recherche
        .route("/search",                       get(search::search))
        // Graphe de connaissances
        .route("/graph",                        get(graph::graph))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state.clone());

    // Route publique (accès à une note partagée sans auth)
    let public = Router::new()
        .route("/public/share/:token", get(public::get_shared_note))
        .with_state(state.clone());

    let system = Router::new()
        .route("/health", get(health::health))
        .with_state(state);

    Router::new()
        .merge(system)
        .merge(public)
        .nest("/", authed)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
