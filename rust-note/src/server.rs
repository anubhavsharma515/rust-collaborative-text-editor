use argon2::{
    password_hash::{
        PasswordHasher, SaltString
    },
    Argon2
};
use axum::{middleware, routing::get, Router};
use std::sync::{Arc, Mutex};
use rand_core::OsRng;
use tokio::task::JoinHandle;

use crate::handlers::{auth, edit_handler, read_handler};

#[derive(Clone)]
pub struct AppState {
    pub read_hash: Option<String>,
    pub content_text: Arc<Mutex<String>>
}

pub async fn start_server(password: Option<String>, content_text: Arc<Mutex<String>>) -> JoinHandle<()> {
    let read_hash = password.and_then(|pass| Some(generate_password_hash(pass)));
    let state = AppState {
        read_hash,
        content_text,
    };

    let app = Router::new()
    .route("/status", get(|| async {
        "UP"
    }))
    .route("/read", get(read_handler))
    .route("/edit", get(edit_handler))
    .layer(middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Server running on: http://localhost:8080");
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap()
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