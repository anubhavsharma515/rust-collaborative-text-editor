use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{extract::{Request, State}, http::{self, StatusCode}, middleware::Next, response::Response};

use crate::server::AppState;

pub async fn read_handler(State(state): State<AppState>) -> String {
    state.content_text.lock().unwrap().to_string()
}

pub async fn edit_handler(State(state): State<AppState>) -> String {
    state.content_text.lock().unwrap().to_string()
}

pub async fn auth(state: State<AppState>, req: Request, next: Next) -> Result<Response, StatusCode> {
    if state.read_hash.is_none() {
        return Ok(next.run(req).await);
    }

    let auth_header = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let parsed_hash = PasswordHash::new(&state.read_hash.as_ref().unwrap()).unwrap();

    if Argon2::default().verify_password(auth_header.as_bytes(), &parsed_hash).is_ok() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}