use crate::{
    editor::CursorMarker,
    handlers::{auth, ws_handler},
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{middleware, routing::get, Router};
use rand_core::OsRng;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

#[derive(Clone)]
pub struct AppState {
    pub read_access_hash: Option<String>,
    pub write_access_hash: Option<String>,
    pub content_text: Arc<Mutex<String>>,
    pub users: Arc<Mutex<Vec<SocketAddr>>>,
    pub user_to_cursor_map: Arc<Mutex<HashMap<SocketAddr, CursorMarker>>>,
}

pub async fn start_server(
    read_access_pass: Option<String>,
    write_access_pass: Option<String>,
    content_text: Arc<Mutex<String>>,
    users: Arc<Mutex<Vec<SocketAddr>>>,
    user_to_cursor_map: Arc<Mutex<HashMap<SocketAddr, CursorMarker>>>,
) -> JoinHandle<()> {
    let read_access_hash = read_access_pass.and_then(|pass| Some(generate_password_hash(pass)));
    let write_access_hash = write_access_pass.and_then(|pass| Some(generate_password_hash(pass)));
    let state = AppState {
        read_access_hash,
        write_access_hash,
        content_text,
        users,
        user_to_cursor_map,
    };

    let app = Router::new()
        .route("/status", get(|| async { "UP" }))
        .route("/read", get(ws_handler))
        .route("/edit", get(ws_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Server running on: http://localhost:8080");
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    })
}

fn generate_password_hash(password: String) -> String {
    let password = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    argon2.hash_password(password, &salt).unwrap().to_string()
}
