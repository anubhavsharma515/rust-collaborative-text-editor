use crate::{
    editor::CursorMarker,
    handlers::{auth, ws_handler},
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{middleware, routing::get, Router};
use cola::{Deletion, EncodedReplica, Replica, ReplicaId};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, ops::Range, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: usize,
    pub cursor: CursorMarker,
}

#[derive(Serialize, Deserialize)]
pub struct Users {
    user_map: HashMap<SocketAddr, User>,
}

impl Users {
    pub fn new() -> Self {
        Self {
            user_map: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, socket_addr: SocketAddr, cursor: CursorMarker) {
        let id = self.user_map.len() + 1;
        self.user_map
            .entry(socket_addr)
            .and_modify(|user| {
                user.cursor = cursor;
            })
            .or_insert(User { id, cursor });
    }

    pub fn get_id(&self, socket_addr: SocketAddr) -> Option<usize> {
        self.user_map.get(&socket_addr).map(|user| user.id)
    }
}

pub struct Document {
    pub buffer: String,
    pub crdt: Replica,
}

#[derive(Serialize, Deserialize)]
pub struct Insertion {
    pub text: String,
    pub crdt: cola::Insertion,
}

impl Insertion {}

#[derive(Serialize, Deserialize)]
pub struct InsertRequest {
    pub insert_at: usize,
    pub text: String,
    pub replica: EncodedReplica,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteRequest {
    pub range: Range<usize>,
    pub document_text: String,
    pub replica: EncodedReplica,
}

// Leveraged from https://docs.rs/cola-crdt/latest/cola/
impl Document {
    pub fn new(buffer: String, replica_id: ReplicaId) -> Self {
        let crdt = Replica::new(replica_id, buffer.len());
        Document { buffer, crdt }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn fork(&self, new_replica_id: ReplicaId) -> Self {
        let crdt = self.crdt.fork(new_replica_id);
        Document {
            buffer: self.buffer.clone(),
            crdt,
        }
    }

    pub fn check_newline_at(&self, index: usize) -> bool {
        self.buffer.chars().nth(index) == Some('\n')
    }

    pub fn insert<S: Into<String>>(&mut self, insert_at: usize, text: S) -> Insertion {
        let text = text.into();
        self.buffer.insert_str(insert_at, &text);
        let insertion = self.crdt.inserted(insert_at, text.len());
        Insertion {
            text,
            crdt: insertion,
        }
    }

    pub fn delete(&mut self, range: Range<usize>) -> Deletion {
        self.buffer.replace_range(range.clone(), "");
        self.crdt.deleted(range)
    }

    pub fn integrate_insertion(&mut self, insertion: Insertion) {
        if let Some(offset) = self.crdt.integrate_insertion(&insertion.crdt) {
            self.buffer.insert_str(offset, &insertion.text);
        }
    }

    pub fn integrate_deletion(&mut self, deletion: Deletion) {
        let ranges = self.crdt.integrate_deletion(&deletion);
        for range in ranges.into_iter().rev() {
            self.buffer.replace_range(range, "");
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub read_access_hash: Option<String>,
    pub write_access_hash: Option<String>,
    pub document: Arc<Mutex<Document>>,
    pub users: Arc<Mutex<Users>>,
}

pub async fn start_server(
    read_access_pass: Option<String>,
    write_access_pass: Option<String>,
    document: Arc<Mutex<Document>>,
    users: Arc<Mutex<Users>>,
) -> JoinHandle<()> {
    let read_access_hash = read_access_pass.and_then(|pass| Some(generate_password_hash(pass)));
    let write_access_hash = write_access_pass.and_then(|pass| Some(generate_password_hash(pass)));
    let state = AppState {
        read_access_hash,
        write_access_hash,
        document,
        users,
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
