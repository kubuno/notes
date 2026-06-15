use crate::config::Settings;
use crate::files_client::FilesClient;
use reqwest::Client;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db:           PgPool,
    pub settings:     Arc<Settings>,
    pub http:         Client,
    pub files_client: Arc<FilesClient>,
}
