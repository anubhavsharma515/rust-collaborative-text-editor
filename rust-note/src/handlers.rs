use crate::{
    editor::CursorMarker,
    server::{AppState, DeleteRequest, InsertRequest, Insertion},
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
use cola::Replica;
use futures::{
    sink::SinkExt,
    stream::{SplitSink, StreamExt},
};
use std::{borrow::Cow, net::SocketAddr, ops::ControlFlow};

const BROADCAST_INTERVAL: u64 = 300;

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

    // Broadcast the content of the document to all clients
    let mut send_task = tokio::spawn(broadcast(sender, state.clone()));

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
            if process_message(msg, who, &mut state).await.is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }

    let (sender, mut receiver) = socket.split();

    // Broadcast the content of the document to all clients
    let mut send_task = tokio::spawn(broadcast(sender, state.clone()));

    // This second task will receive messages from client and print them on server console
    let mut recv_task = tokio::spawn(async move {
        let mut cnt = 0;
        while let Some(Ok(msg)) = receiver.next().await {
            cnt += 1;

            if process_message(msg, who, &mut state).await.is_break() {
                break;
            }
        }
        cnt
    });

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
}

async fn broadcast(mut sender: SplitSink<WebSocket, Message>, state: AppState) -> i32 {
    let mut n_msg = 0;
    loop {
        let doc = state.document.lock().await;
        if sender
            .send(Message::Text(format!("Document: {}", doc.buffer)))
            .await
            .is_err()
        {
            break;
        }
        n_msg += 1;

        let users = state.users.lock().await;
        let users_json = serde_json::to_string(&*users).unwrap();
        if sender
            .send(Message::Text(format!("Users: {}", users_json)))
            .await
            .is_err()
        {
            break;
        }
        n_msg += 1;

        tokio::time::sleep(std::time::Duration::from_millis(BROADCAST_INTERVAL)).await;
    }

    println!("Broadcasting close...");
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
    msg: Message,
    who: SocketAddr,
    state: &mut AppState,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {t:?}");
            let parts: Vec<&str> = t.split(":").collect();
            let mut iter = parts.into_iter();
            match iter.next() {
                Some("Insert") => {
                    let s = iter.collect::<Vec<&str>>().join(":");
                    match serde_json::from_str::<InsertRequest>(s.trim()) {
                        Ok(insert) => {
                            if let Some(id) = state.users.lock().await.get_id(who) {
                                let mut fork = Replica::decode(id as u64, &insert.replica).unwrap();
                                let insertion = fork.inserted(insert.insert_at, insert.text.len());
                                state.document.lock().await.integrate_insertion(Insertion {
                                    text: insert.text,
                                    crdt: insertion,
                                });
                            }
                        }
                        Err(e) => println!("Error parsing insert: {e}"),
                    }
                }
                Some("Delete") => {
                    let s = iter.collect::<Vec<&str>>().join(":");
                    match serde_json::from_str::<DeleteRequest>(s.trim()) {
                        Ok(delete) => {
                            if let Some(id) = state.users.lock().await.get_id(who) {
                                let mut fork = Replica::decode(id as u64, &delete.replica).unwrap();
                                let deletion = fork.deleted(delete.range);
                                state.document.lock().await.integrate_deletion(deletion);
                            }
                        }
                        Err(e) => println!("Error parsing delete: {e}"),
                    }
                }
                Some("Cursor") => {
                    let s = iter.collect::<Vec<&str>>().join(":");
                    match serde_json::from_str::<CursorMarker>(s.trim()) {
                        Ok(cursor) => state.users.lock().await.add_user(who, cursor),
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
            return ControlFlow::Break(());
        }

        Message::Pong(v) => {
            println!(">>> {who} sent pong with {v:?}");
        }
        Message::Ping(v) => {
            println!(">>> {who} sent ping with {v:?}");
        }
    }
    ControlFlow::Continue(())
}
