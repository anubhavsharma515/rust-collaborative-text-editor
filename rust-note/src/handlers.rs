use crate::{
    editor::CursorMarker,
    server::{AppState, Deletion, Insertion, Operation},
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    body::Body,
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        ConnectInfo, Request, State, WebSocketUpgrade,
    },
    http::{self, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::TypedHeader;
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use std::{borrow::Cow, net::SocketAddr};
use tokio::sync::broadcast::Receiver;

pub async fn auth(
    state: State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let parsed_hash = match req.uri().path() {
        "/read" => {
            if state.read_access_hash.is_none() {
                return Ok(next.run(req).await);
            }

            PasswordHash::new(&state.read_access_hash.as_ref().unwrap()).unwrap()
        }
        "/edit" => {
            if state.write_access_hash.is_none() {
                return Ok(next.run(req).await);
            }

            PasswordHash::new(&state.write_access_hash.as_ref().unwrap()).unwrap()
        }
        _ => return Ok(next.run(req).await),
    };

    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if Argon2::default()
        .verify_password(auth_header.as_bytes(), &parsed_hash)
        .is_ok()
    {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn ws_handler(
    state: State<AppState>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");

    match req.uri().path() {
        "/read" => ws.on_upgrade(move |socket| handle_read_socket(socket, addr, state)),
        "/edit" => ws.on_upgrade(move |socket| handle_edit_socket(socket, addr, state)),
        _ => {
            let res = Response::new(Body::empty());
            let (mut parts, body) = res.into_parts();

            parts.status = StatusCode::NOT_FOUND;
            Response::from_parts(parts, body)
        }
    }
}

async fn handle_read_socket(socket: WebSocket, who: SocketAddr, State(state): State<AppState>) {
    let (sender, _) = socket.split();

    let rx = state.tx.subscribe();

    // Broadcast the content of the document to all clients
    let mut send_task = tokio::spawn(broadcast(sender, rx, who, state.clone()));

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => println!("{a} messages sent to {who}"),
                Err(a) => println!("Error sending messages {a:?}")
            }
        },
    }

    println!("Websocket context {who} destroyed");
}

async fn handle_edit_socket(
    mut socket: WebSocket,
    who: SocketAddr,
    State(mut state): State<AppState>,
) {
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {who}...");
    } else {
        println!("Could not send ping {who}!");
        return;
    }

    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Pong(v) => {
                    println!(">>> {who} sent pong with {v:?}");
                }
                _ => {
                    println!("client {who} did not pong my ping");
                    return;
                }
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }

    let (sender, receiver) = socket.split();

    let rx = state.tx.subscribe();

    // Broadcast the content of the document to client
    let mut send_task = tokio::spawn(broadcast(sender, rx, who, state.clone()));

    // This second task will receive messages from client
    let mut recv_task = tokio::spawn(process_message(receiver, who, state.clone()));

    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(a) => println!("{a} messages sent to {who}"),
                Err(a) => println!("Error sending messages {a:?}")
            }
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(b) => println!("Received {b} messages"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
        }
    }

    println!("Websocket context {who} destroyed");
    // Remove user from the list of users
    let mut users = state.users.lock().await;
    users.remove_user(who);
    *state.is_moved.lock().await = true;

    let cursors = users.get_all_cursors();
    state
        .server_worker
        .send(crate::editor::Input::Cursors(cursors))
        .await
        .unwrap();
}

async fn broadcast(
    mut sender: SplitSink<WebSocket, Message>,
    mut rx: Receiver<String>,
    who: SocketAddr,
    state: AppState,
) -> i32 {
    let mut n_msg = 0;

    // Send the document, cursors, and the client's id to the client that just connected
    // This is the first message that the client will receive
    {
        let doc = state.document.lock().await;
        let mut users = state.users.lock().await;
        // Get the id of the user, if it does not exist, add it
        let id = users
            .get_id(who)
            .unwrap_or_else(|| users.add_user(who, None)) as u64;

        let doc_json = serde_json::to_string(&*doc).unwrap();
        if sender
            .send(Message::Text(format!("Document: {}", doc_json)))
            .await
            .is_err()
        {
            return n_msg;
        }

        if sender
            .send(Message::Text(format!("Id: {}", id)))
            .await
            .is_err()
        {
            return n_msg;
        }

        let users_json = serde_json::to_string(&*users).unwrap();
        if sender
            .send(Message::Text(format!("Users: {}", users_json)))
            .await
            .is_err()
        {
            return n_msg;
        }

        println!("New client connected, document, id and cursors sent to {who}");
        n_msg += 3;
    }

    // Forward the broadcasts to the client
    while let Ok(msg) = rx.recv().await {
        if sender.send(Message::Text(msg)).await.is_err() {
            break;
        }
        n_msg += 1;
    }

    println!("Channel closed...");
    if let Err(e) = sender
        .send(Message::Close(Some(CloseFrame {
            code: axum::extract::ws::close_code::NORMAL,
            reason: Cow::from("Goodbye"),
        })))
        .await
    {
        println!("Could not send Close due to {e}, probably it is ok?");
    }
    n_msg
}

async fn process_message(
    mut receiver: SplitStream<WebSocket>,
    who: SocketAddr,
    mut state: AppState,
) -> i32 {
    let mut n_msg = 0;
    while let Some(Ok(msg)) = receiver.next().await {
        n_msg += 1;

        match msg {
            Message::Text(t) => {
                println!(">>> {who} sent str: {t:?}");
                let parts: Vec<&str> = t.split(":").collect();
                let mut iter = parts.into_iter();
                match iter.next() {
                    Some("Insert") => {
                        let s = iter.collect::<Vec<&str>>().join(":");
                        match serde_json::from_str::<Insertion>(s.trim()) {
                            Ok(insertion) => {
                                if let Some(id) = state.users.lock().await.get_id(who) {
                                    let mut doc = state.document.lock().await;
                                    doc.last_edit = id;
                                    doc.insert(insertion.insert_at, insertion.clone().text);

                                    *state.is_dirty.lock().await = true;
                                }
                            }
                            Err(e) => println!("Error parsing insert: {e}"),
                        }
                    }
                    Some("Delete") => {
                        let s = iter.collect::<Vec<&str>>().join(":");
                        match serde_json::from_str::<Deletion>(s.trim()) {
                            Ok(deletion) => {
                                if let Some(id) = state.users.lock().await.get_id(who) {
                                    let mut doc = state.document.lock().await;
                                    doc.last_edit = id;
                                    doc.delete(deletion.clone().range);

                                    *state.is_dirty.lock().await = true;
                                }
                            }
                            Err(e) => println!("Error parsing delete: {e}"),
                        }
                    }
                    Some("Cursor") => {
                        let s = iter.collect::<Vec<&str>>().join(":");
                        match serde_json::from_str::<CursorMarker>(s.trim()) {
                            Ok(cursor) => {
                                let mut users = state.users.lock().await;
                                users.add_user(who, Some(cursor));
                                *state.is_moved.lock().await = true;

                                let cursors = users.get_all_cursors();
                                state
                                    .server_worker
                                    .send(crate::editor::Input::Cursors(cursors))
                                    .await
                                    .unwrap();
                            }
                            Err(e) => println!("Error parsing cursor: {e}"),
                        }
                    }
                    _ => {}
                }
            }
            Message::Binary(d) => {
                println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
            }
            Message::Close(c) => {
                if let Some(cf) = c {
                    println!(
                        ">>> {} sent close with code {} and reason `{}`",
                        who, cf.code, cf.reason
                    );
                } else {
                    println!(">>> {who} somehow sent close message without CloseFrame");
                }
                break;
            }

            Message::Pong(v) => {
                println!(">>> {who} sent pong with {v:?}");
            }
            Message::Ping(v) => {
                println!(">>> {who} sent ping with {v:?}");
            }
        }
    }

    n_msg
}
