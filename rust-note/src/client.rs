use axum::http::Request;
use iced::futures;
use iced::stream;
use iced::widget::text;

use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::stream::{Stream, StreamExt};

use async_tungstenite::tungstenite;
use reqwest;
use std::fmt;

pub fn connect(access: String, pass: String) -> impl Stream<Item = Event> {
    stream::channel(100, |mut output| async move {
        let mut state = State::Disconnected;

        loop {
            match &mut state {
                State::Disconnected => {
                    let status_endpoint = "http://0.0.0.0:8080/status";
                    let client = reqwest::Client::new();

                    let resp = client.get(status_endpoint).send().await;

                    if resp.is_err() {
                        let _ = output.send(Event::ServerDown).await;
                        continue;
                    }

                    let url = format!("ws://0.0.0.0:8080/{}", access);
                    let request = Request::builder()
                        .uri(url)
                        .header("AUTHORIZATION", pass.clone())
                        .header("sec-websocket-key", "foo")
                        .header("upgrade", "websocket")
                        .header("host", "server.example.com")
                        .header("connection", "upgrade")
                        .header("sec-websocket-version", 13)
                        .body(())
                        .unwrap();

                    match async_tungstenite::tokio::connect_async(request).await {
                        Ok((websocket, _)) => {
                            // Split the websocket into a channel for seding and receiving messages
                            let (sender, receiver) = mpsc::channel(100);

                            let _ = output.send(Event::Connected(Connection(sender))).await;

                            state = State::Connected(websocket, receiver);
                        }
                        //try and get more granular here with the event that's being fired back
                        Err(err) => {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            match err {
                                tungstenite::Error::Http(code) => {
                                    let status = code.status();
                                    if status == 401 {
                                        let _ = output.send(Event::IncorrectPassword).await;
                                        continue;
                                    }
                                }
                                _ => {}
                            }

                            let _ = output.send(Event::Disconnected).await;
                        }
                    }
                }
                State::Connected(websocket, input) => {
                    let mut fused_websocket = websocket.by_ref().fuse();

                    // Run the tasks concurrently
                    futures::select! {
                        received = fused_websocket.select_next_some() => {
                            // Receive the message from the websocket
                            match received {
                                Ok(tungstenite::Message::Text(message)) => {
                                   let _ = output.send(Event::MessageReceived(Message::User(message))).await;
                                }
                                Err(_) => {
                                    let _ = output.send(Event::Disconnected).await;

                                    state = State::Disconnected;
                                }
                                Ok(_) => continue,
                            }
                        }

                        message = input.select_next_some() => {
                            match message {
                                Message::CloseConnection => {
                                    // Close the WebSocket connection gracefully
                                    let _ = websocket.close(None).await;
                                    let _ = output.send(Event::Disconnected).await;
                                }
                                other_message => {
                                    // Send other messages to the WebSocket server
                                    let result = websocket.send(tungstenite::Message::Text(other_message.to_string())).await;

                                    if result.is_err() {
                                        let _ = output.send(Event::Disconnected).await;

                                        state = State::Disconnected;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum State {
    Disconnected,
    Connected(
        // Simply a connection to the websocket
        async_tungstenite::WebSocketStream<async_tungstenite::tokio::ConnectStream>,
        mpsc::Receiver<Message>,
    ),
}

#[derive(Debug, Clone)]
pub enum Event {
    Connected(Connection),
    Disconnected,
    MessageReceived(Message),
    ServerDown,
    IncorrectPassword, //Add a more granular variant that maps whether there's a success or failure
}

#[derive(Debug, Clone)]
pub struct Connection(mpsc::Sender<Message>);

impl Connection {
    pub fn send(&mut self, message: Message) {
        self.0
            .try_send(message)
            .expect("Send message to echo server");
    }
    pub fn close(&mut self) {
        self.send(Message::CloseConnection);
    }
}

// Check if this needs to be an axum ws message
// Will need to be able to parse the message
#[derive(Debug, Clone)]
pub enum Message {
    Connected,
    Disconnected,
    User(String),
    CloseConnection,
}

impl Message {
    pub fn new(message: &str) -> Option<Self> {
        if message.is_empty() {
            None
        } else {
            Some(Self::User(message.to_string()))
        }
    }

    pub fn connected() -> Self {
        Message::Connected
    }

    pub fn disconnected() -> Self {
        Message::Disconnected
    }

    pub fn as_str(&self) -> &str {
        match self {
            Message::Connected => "Connected successfully!",
            Message::Disconnected => "Connection lost... Retrying...",
            Message::User(message) => message.as_str(),
            Message::CloseConnection => "Closing Connection",
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> text::IntoFragment<'a> for &'a Message {
    fn into_fragment(self) -> text::Fragment<'a> {
        text::Fragment::Borrowed(self.as_str())
    }
}
