use crate::{
    editor::{CursorMarker, Input},
    handlers::{auth, ws_handler},
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{middleware, routing::get, Router};
use cola::{EncodedReplica, Replica, ReplicaId};
use futures::channel::mpsc;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, ops::Range, sync::Arc};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: usize,
    pub cursor: Option<CursorMarker>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Users {
    user_map: HashMap<SocketAddr, User>,
}

impl Users {
    pub fn new() -> Self {
        Self {
            user_map: HashMap::new(),
        }
    }

    pub fn add_user(&mut self, socket_addr: SocketAddr, cursor: Option<CursorMarker>) -> usize {
        let id = self.user_map.len() + 1;
        self.user_map
            .entry(socket_addr)
            .and_modify(|user| {
                user.cursor = cursor;
            })
            .or_insert(User { id, cursor });
        id
    }

    pub fn get_id(&self, socket_addr: SocketAddr) -> Option<usize> {
        self.user_map.get(&socket_addr).map(|user| user.id)
    }

    pub fn get_all_cursors(&self) -> Vec<CursorMarker> {
        self.user_map
            .values()
            .filter_map(|user| user.cursor.clone())
            .collect()
    }

    pub fn remove_user(&mut self, socket_addr: SocketAddr) {
        self.user_map.remove(&socket_addr);
    }

    pub fn delete_all_users(&mut self) {
        self.user_map.clear();
    }
}

#[derive(Debug)]
pub struct Document {
    pub buffer: String,
    pub crdt: Replica,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentTransmit {
    pub id: u64,
    pub text: String,
    pub replica: EncodedReplica,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insertion {
    pub insert_at: usize,
    pub text: String,
    pub crdt: cola::Insertion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deletion {
    pub range: Range<usize>,
    pub crdt: cola::Deletion,
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

    pub fn check_newline_at(&self, index: usize) -> bool {
        self.buffer.chars().nth(index) == Some('\n')
    }

    pub fn insert<S: Into<String>>(&mut self, insert_at: usize, text: S) -> Insertion {
        let text = text.into();
        self.buffer.insert_str(insert_at, &text);
        let insertion = self.crdt.inserted(insert_at, text.len());
        Insertion {
            insert_at,
            text,
            crdt: insertion,
        }
    }

    pub fn delete(&mut self, range: Range<usize>) -> Deletion {
        self.buffer.replace_range(range.clone(), "");
        let deletion = self.crdt.deleted(range.clone());
        Deletion {
            range,
            crdt: deletion,
        }
    }

    pub fn integrate_insertion(&mut self, insertion: Insertion) {
        dbg!(&insertion);

        if let Some(offset) = self.crdt.integrate_insertion(&insertion.crdt) {
            self.buffer.insert_str(offset, &insertion.text);
        }
    }

    pub fn integrate_deletion(&mut self, deletion: Deletion) {
        dbg!(&deletion);

        let ranges = self.crdt.integrate_deletion(&deletion.crdt);
        for range in ranges.into_iter().rev() {
            self.buffer.replace_range(range, "");
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    Insert(Insertion),
    Delete(Deletion),
}

#[derive(Clone)]
pub struct AppState {
    pub read_access_hash: Option<String>,
    pub write_access_hash: Option<String>,
    pub document: Arc<Mutex<Document>>,
    pub is_dirty: Arc<Mutex<bool>>,
    pub users: Arc<Mutex<Users>>,
    pub is_moved: Arc<Mutex<bool>>,
    pub server_worker: mpsc::Sender<Input>,
    pub tx: broadcast::Sender<String>,
    pub operations: Arc<Mutex<Vec<Operation>>>,
}

pub async fn start_server(
    read_access_pass: Option<String>,
    write_access_pass: Option<String>,
    document: Arc<Mutex<Document>>,
    is_dirty: Arc<Mutex<bool>>,
    users: Arc<Mutex<Users>>,
    is_moved: Arc<Mutex<bool>>,
    server_worker: mpsc::Sender<Input>,
    operations: Arc<Mutex<Vec<Operation>>>,
) -> JoinHandle<()> {
    let read_access_hash = read_access_pass.and_then(|pass| Some(generate_password_hash(pass)));
    let write_access_hash = write_access_pass.and_then(|pass| Some(generate_password_hash(pass)));
    let (tx, _rx) = broadcast::channel(100);

    let state = AppState {
        read_access_hash,
        write_access_hash,
        document,
        is_dirty,
        users,
        is_moved,
        server_worker,
        tx,
        operations,
    };

    // Continuously broadcast any operations to the clients
    let state_copy = state.clone();
    tokio::spawn(async move {
        let state = state_copy;

        loop {
            if *state.is_dirty.lock().await && state.tx.receiver_count() > 0 {
                let mut operations = state.operations.lock().await;
                for operation in operations.iter() {
                    state
                        .tx
                        .send(format!(
                            "Edit: {}",
                            serde_json::to_string(operation).unwrap()
                        ))
                        .unwrap();
                }
                operations.clear();
                *state.is_dirty.lock().await = false;
            }

            if *state.is_moved.lock().await {
                let users = state.users.lock().await;
                let users_json = serde_json::to_string(&*users).unwrap();
                state.tx.send(format!("Users: {}", users_json)).unwrap();
                *state.is_moved.lock().await = false;
            }
        }
    });

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
