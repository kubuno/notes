use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{errors::NotesError, state::AppState};

#[derive(Debug, Clone)]
pub struct NotesUser {
    pub id:    Uuid,
    pub role:  String,
    pub email: String,
}

pub type NotesUserExt = axum::Extension<NotesUser>;

/// This module's id, used as the token audience: a token minted for another
/// module does not validate here.
const MODULE_ID: &str = "notes";

/// Middleware: authenticate the caller from the signed `X-Kubuno-Auth` token the
/// core mints with this module's internal secret (see `kubuno-modauth`), instead
/// of trusting the plain `X-Kubuno-User-*` headers — which any process reaching
/// this module's loopback port could otherwise forge to impersonate any user.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, NotesError> {
    let token = req
        .headers()
        .get(kubuno_modauth::TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(NotesError::Unauthorized)?;

    let user = kubuno_modauth::verify(
        state.settings.core.internal_secret.as_bytes(),
        token,
        MODULE_ID,
    )
    .map_err(|_| NotesError::Unauthorized)?;

    req.extensions_mut()
        .insert(NotesUser { id: user.id, role: user.role, email: user.email });
    Ok(next.run(req).await)
}
