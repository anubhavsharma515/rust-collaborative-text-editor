use crate::{
    client,
    server::{start_server, Document, Operation, UserId, Users},
    widgets,
};
use futures::{channel::mpsc, SinkExt, Stream};
use iced::{
    highlighter, keyboard, mouse, stream,
    widget::{
        button,
        canvas::{self, Frame, Path as icedPath},
        center, column, container, horizontal_space, markdown, mouse_area, opaque, radio, row,
        scrollable, stack, text, text_editor, text_input, toggler, Canvas, Container, Stack, Text,
        TextEditor,
    },
    window, Alignment, Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size,
    Subscription, Task, Theme,
};
use iced_aw::{TabLabel, Tabs};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    ffi, fmt,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use widgets::format_bar::{FormatBar, TextStyle, DEFAULT_FONT_SIZE};
use widgets::menubar::{load_file, open_file, save_file, MenuBar, MenuMessage}; // For form parameters

const BOLD_HOTKEY: &str = "b";
const ITALIC_HOTKEY: &str = "i";
const STRIKETHROUGH_HOTKEY: &str = "f";
const SHORTCUT_PALETTE_HOTKEY: &str = "p";
const SESSION_MODAL_HOTKEY: &str = "n";

#[derive(Clone)]
pub struct SessionModal {
    pub session_password_input: String,
    pub write_password_input: String,
    pub read_password_input: String,
    pub file_path_input: String,
    pub file_error: String,
    pub session_join_error: String,
    pub session_selection: Option<SessionType>,
}

impl Default for SessionModal {
    fn default() -> Self {
        Self {
            session_password_input: String::new(),
            write_password_input: String::new(),
            read_password_input: String::new(),
            file_path_input: String::new(),
            file_error: String::new(),
            session_join_error: String::new(),
            session_selection: Some(SessionType::Read),
        }
    }
}

impl SessionModal {
    pub fn validate_password(&self) -> bool {
        if self.read_password_input.is_empty() && self.write_password_input.len() >= 1 {
            true
        } else if self.read_password_input.len() >= 1 && self.write_password_input.is_empty() {
            true
        } else if self.read_password_input.len() >= 1 && self.write_password_input.len() >= 1 {
            true
        } else {
            false
        }
    }
    pub fn validate_file(&mut self) -> bool {
        if !&self.file_path_input.is_empty() {
            if self.file_path_input.ends_with(".md")
                && std::path::Path::new(&self.file_path_input).exists()
            {
                self.file_error = "".to_string();
                true
            } else {
                self.file_error = "Invalid Markdown file path.".to_string();
                false
            }
        } else {
            self.file_error = "".to_string(); // File is optional
            true
        }
    }
}

pub struct Editor {
    content: text_editor::Content,
    document: Arc<Mutex<Document>>,
    is_dirty: Arc<Mutex<bool>>,
    cursor_marker: CursorMarker,
    is_moved: Arc<Mutex<bool>>,
    menubar: MenuBar,
    format_bar: FormatBar,
    file: Option<PathBuf>,
    theme: Theme,
    markdown_text: Vec<markdown::Item>,
    markdown_settings: markdown::Settings,
    modal_content: SessionModal,
    markdown_preview_open: bool,
    shortcut_palette_open: bool,
    session_modal_open: bool,
    active_tab: TabId,
    server_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    users: Arc<Mutex<Users>>,
    user_cursors: Vec<CursorMarker>,
    read_password: Option<String>,
    edit_password: Option<String>,
    joined_session: bool,
    leave_session: bool,
    started_session: bool,
    client_state: State,
    id: Option<UserId>, // Id for collab sessions
    server_worker: Option<mpsc::Sender<Input>>,
}

enum State {
    Disconnected,
    Connected(client::Connection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Read,
    Edit,
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            SessionType::Read => "read",
            SessionType::Edit => "edit",
        };
        write!(f, "{}", value)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Action(text_editor::Action),
    Menu(MenuMessage),
    Format(TextStyle),
    LinkClicked(markdown::Url),
    ShowMarkdownPreview(bool),
    NoOp,
    DeleteLine,
    DeleteWord,
    ShortcutPaletteToggle,
    SessionModalToggle,
    SessionPasswordChanged(String),
    WritePasswordChanged(String),
    ReadPasswordChanged(String),
    FilePathChanged(String),
    StartSessionPressed,
    UpdateHostDoc(Document),
    UpdateHostCursors(Vec<CursorMarker>),
    JoinSessionPressed,
    TabSelected(TabId),
    Echo(client::Event),
    RequestClose,
    LeaveSession,
    SessionClosed,
    SessionTypeRequested(SessionType),
    CloseWindow(iced::window::Id),
    WorkerReady(mpsc::Sender<Input>),
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum TabId {
    #[default]
    StartSession,
    JoinSession,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CursorMarker {
    pub y: f32,
    pub color: (f32, f32, f32),
}

impl CursorMarker {
    pub fn new(y: f32) -> Self {
        let mut rng = rand::thread_rng();

        // Generate random RGB values
        let r = rng.gen_range(0.0..=1.0);
        let g = rng.gen_range(0.0..=1.0);
        let b = rng.gen_range(0.0..=1.0);
        Self {
            y,
            color: (r, g, b),
        }
    }

    pub fn move_cursor(&mut self, y: f32) {
        self.y = y;
    }
}

impl<Message> canvas::Program<Message> for CursorMarker {
    // No internal state
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        // let offset_x = 2.0; // Offset for padding/margin adjustments
        // let offset_y = 2.0; // Offset for padding/margin adjustments

        let rectangle = icedPath::rectangle(Point::new(0.0, self.y), Size::new(5.5, 21.0));
        frame.fill(
            &rectangle,
            Color::from_rgb(self.color.0, self.color.1, self.color.2),
        );
        vec![frame.into_geometry()]
    }
}

impl Editor {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                document: Arc::new(Mutex::new(Document::new(String::new()))),
                is_dirty: Arc::new(Mutex::new(false)),
                cursor_marker: CursorMarker::new(0.2),
                is_moved: Arc::new(Mutex::new(false)),
                menubar: MenuBar::new(),
                format_bar: FormatBar::new(),
                file: None,
                theme: Theme::default(),
                modal_content: SessionModal::default(),
                markdown_text: markdown::parse("Write your **Markdown** text here.").collect(),
                markdown_settings: markdown::Settings::with_text_size(DEFAULT_FONT_SIZE),
                markdown_preview_open: false,
                shortcut_palette_open: false,
                session_modal_open: false,
                active_tab: TabId::StartSession,
                server_thread: Arc::new(Mutex::new(None)),
                users: Arc::new(Mutex::new(Users::new())),
                user_cursors: Vec::new(),
                joined_session: false,
                started_session: false,
                leave_session: false,
                read_password: None,
                edit_password: None,
                client_state: State::Disconnected,
                id: None,
                server_worker: None,
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        String::from("rust-note")
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let subscriptions = vec![
            window::events().map(|(id, evt)| match evt {
                iced::window::Event::CloseRequested => Message::CloseWindow(id),
                _ => Message::NoOp,
            }),
            if self.joined_session {
                let session_type_str = self.modal_content.session_selection.unwrap().to_string(); // Convert `SessionType` to `String` if `Some`

                Subscription::run_with_id(
                    "id",
                    client::connect(
                        session_type_str, // Pass the resolved string
                        self.modal_content.session_password_input.clone(),
                    ),
                )
                .map(Message::Echo)
            } else {
                Subscription::none()
            },
            Subscription::run(server_worker),
        ];

        Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut markdown_settings = markdown::Settings::default();
        markdown_settings.text_size = iced::Pixels(50.0);

        let status = row![
            {
                let button = if self.started_session {
                    button("Stop Session")
                        .on_press(Message::RequestClose)
                        .style(button::primary)
                } else if self.joined_session {
                    button("Leave Session")
                        .on_press(Message::LeaveSession)
                        .style(button::primary)
                } else {
                    button("Collaborate")
                        .on_press(Message::SessionModalToggle)
                        .style(button::primary)
                };
                button
            },
            text(if let Some(path) = &self.file {
                let path = path.display().to_string();

                if path.len() > 60 {
                    format!("...{}", &path[path.len() - 40..])
                } else {
                    path
                }
            } else {
                String::from("")
            }),
            horizontal_space(),
            text({
                let (line, column) = self.content.cursor_position();
                let content = self.content.text();
                let words = &content.split(" ").count();
                let lines = &content.split("\n").count();

                format!(
                    "Words: {} | Lines: {} | Line {}, Columns {}",
                    words - 1,
                    lines - 1,
                    line + 1,
                    column + 1
                )
            })
        ]
        .spacing(10);

        let shortcut_palette: Container<Message> = container(
            column![
                text("Shortcut Map").size(24),
                column![
                    Text::new(format!("cmd + {BOLD_HOTKEY}: Bold")),
                    Text::new(format!("cmd + {ITALIC_HOTKEY}: Italic")),
                    Text::new(format!("cmd + {STRIKETHROUGH_HOTKEY}: Strikethrough")),
                    Text::new("cmd + option + backspace: Delete word"),
                    Text::new("cmd + backspace: Delete line"),
                    Text::new(format!(
                        "cmd + {SHORTCUT_PALETTE_HOTKEY}: Toggle shortcut palette"
                    )),
                    Text::new(format!(
                        "cmd + {SESSION_MODAL_HOTKEY}: Toggle session modal"
                    )),
                ]
                .spacing(10)
            ]
            .spacing(20),
        )
        .width(300)
        .padding(10)
        .style(container::rounded_box);

        let session_modal: Container<Message> = container(
            column![
                text("Colab").size(24),
                Tabs::new(Message::TabSelected)
                    .push(
                        TabId::StartSession,
                        TabLabel::Text(String::from("Start Session")),
                        column![
                            text_input(
                                "Enter file path (optional)",
                                &self.modal_content.file_path_input.clone()
                            )
                            .on_input(Message::FilePathChanged)
                            .padding(5),
                            if !self.modal_content.clone().validate_file() {
                                text("Invalid Markdown file path")
                                    .size(14)
                                    .color([1.0, 0.0, 0.0])
                            } else {
                                text("").size(14)
                            },
                            row![
                                text_input(
                                    "Enter read session password",
                                    &self.modal_content.read_password_input
                                )
                                .on_input(Message::ReadPasswordChanged)
                                .padding(5),
                                text_input(
                                    "Enter write session password",
                                    &self.modal_content.write_password_input
                                )
                                .on_input(Message::WritePasswordChanged)
                                .padding(5),
                            ],
                            {
                                let mut button = button("Start Session").style(button::secondary);
                                if self.modal_content.validate_password()
                                    && ((!self.modal_content.file_path_input.clone().is_empty()
                                        && self.modal_content.clone().validate_file())
                                        || self.modal_content.file_path_input.clone().is_empty())
                                {
                                    button = button
                                        .on_press(Message::StartSessionPressed)
                                        .style(button::primary);
                                }
                                button
                            }
                        ]
                        .spacing(10)
                        .padding(10)
                    )
                    .push(
                        TabId::JoinSession,
                        TabLabel::Text(String::from("Join Session")),
                        column![
                            text_input(
                                "Enter session password",
                                &self.modal_content.session_password_input
                            )
                            .on_input(Message::SessionPasswordChanged)
                            .padding(5),
                            row![
                                radio(
                                    "Read Session",
                                    SessionType::Read,
                                    self.modal_content.session_selection,
                                    Message::SessionTypeRequested
                                ),
                                radio(
                                    "Write Session",
                                    SessionType::Edit,
                                    self.modal_content.session_selection,
                                    Message::SessionTypeRequested
                                ),
                            ]
                            .spacing(10),
                            {
                                let mut button = button("Join Session").style(button::secondary);
                                if self.modal_content.session_password_input.len() > 0 {
                                    button = button
                                        .on_press(Message::JoinSessionPressed)
                                        .style(button::primary);
                                }
                                button
                            },
                            if !self.modal_content.session_join_error.is_empty() {
                                text(&self.modal_content.session_join_error)
                                    .size(14)
                                    .color([1.0, 0.0, 0.0])
                            } else {
                                text("").size(14)
                            },
                        ]
                        .spacing(10)
                        .padding(10)
                    )
                    .tab_label_padding(10)
                    .set_active_tab(&self.active_tab)
            ]
            .padding(10)
            .spacing(20),
        )
        .height(Length::Shrink)
        .width(Length::Shrink)
        .padding(10)
        .style(container::rounded_box);

        let editor = TextEditor::new(&self.content)
            .line_height(text::LineHeight::Absolute(Pixels(21.0)))
            .highlight(
                self.file
                    .as_deref()
                    .and_then(Path::extension)
                    .and_then(ffi::OsStr::to_str)
                    .unwrap_or("md"),
                highlighter::Theme::SolarizedDark,
            )
            .wrapping(text::Wrapping::WordOrGlyph)
            .width(300)
            .height(Length::FillPortion(1))
            .on_action(Message::Action)
            .key_binding(|key_press| match key_press.key.as_ref() {
                keyboard::Key::Character(BOLD_HOTKEY) if key_press.modifiers.command() => Some(
                    text_editor::Binding::Custom(Message::Format(TextStyle::Bold)),
                ),
                keyboard::Key::Character(ITALIC_HOTKEY) if key_press.modifiers.command() => Some(
                    text_editor::Binding::Custom(Message::Format(TextStyle::Italic)),
                ),
                keyboard::Key::Character(STRIKETHROUGH_HOTKEY) if key_press.modifiers.command() => {
                    Some(text_editor::Binding::Custom(Message::Format(
                        TextStyle::Strikethrough,
                    )))
                }
                keyboard::Key::Named(keyboard::key::Named::Backspace)
                    if key_press.modifiers.command() =>
                {
                    if key_press.modifiers.alt() {
                        Some(text_editor::Binding::Custom(Message::DeleteWord))
                    } else {
                        Some(text_editor::Binding::Custom(Message::DeleteLine))
                    }
                }
                keyboard::Key::Character(SHORTCUT_PALETTE_HOTKEY)
                    if key_press.modifiers.command() =>
                {
                    Some(text_editor::Binding::Custom(Message::ShortcutPaletteToggle))
                }
                keyboard::Key::Character(SESSION_MODAL_HOTKEY) if key_press.modifiers.command() => {
                    Some(text_editor::Binding::Custom(Message::SessionModalToggle))
                }
                _ => text_editor::Binding::from_key_press(key_press),
            });

        let mut marker_elements: Vec<Element<Message>> = self
            .user_cursors
            .clone()
            .into_iter()
            .map(|marker| {
                // Create a Canvas for each marker and convert it to an Element
                Canvas::<CursorMarker, Message>::new(marker)
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1))
                    .into() // Convert the Canvas into an Element<Message>
            })
            .collect();

        let mut stack_elements = Vec::new();
        stack_elements.push(editor.into());
        stack_elements.append(&mut marker_elements);

        // println!("Marker elements: {:?}", marker_elements);

        let content = column![
            row![
                self.menubar
                    .view(
                        self.theme.clone(),
                        if let State::Connected(_) = self.client_state {
                            // Use `connection` here
                            true
                        } else {
                            false
                        }
                    )
                    .map(Message::Menu),
                toggler(self.markdown_preview_open)
                    .label("Show Markdown preview")
                    .on_toggle(Message::ShowMarkdownPreview)
            ]
            .spacing(15),
            self.format_bar.view().map(Message::Format),
            row![
                Stack::with_children(stack_elements)
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1)),
                if self.markdown_preview_open {
                    scrollable(
                        markdown::view(
                            &self.markdown_text,
                            self.markdown_settings,
                            markdown::Style::from_palette(self.theme.clone().palette()),
                        )
                        .map(Message::LinkClicked),
                    )
                    .width(Length::FillPortion(1))
                    .height(Length::FillPortion(1))
                } else {
                    scrollable(column![]).width(Length::Shrink)
                },
            ]
            .spacing(20)
            .align_y(Alignment::Start),
            status, // Add the status widget here
        ]
        .align_x(Alignment::Center)
        .spacing(10);

        if self.shortcut_palette_open {
            modal(content, shortcut_palette, Message::ShortcutPaletteToggle)
        } else if self.session_modal_open {
            modal(content, session_modal, Message::SessionModalToggle)
        } else {
            content.into()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Action(action) => {
                let (x, y) = self.content.cursor_position();
                let mut is_blank_line = false;
                let mut running_sum = 0;
                let mut running_sum_vec = vec![0];
                let lines = self.content.lines();
                lines.enumerate().for_each(|(idx, line)| {
                    if idx == x {
                        is_blank_line = line.is_empty();
                    }
                    running_sum += line.len() + 1;
                    running_sum_vec.push(running_sum);
                });

                let mut connection = if let State::Connected(ref mut connection) = self.client_state
                {
                    Some(connection.clone())
                } else {
                    None
                };

                let content_text = self.content.text();
                let mut index = running_sum_vec.get(x).unwrap().clone() + y;

                let doc_lock = self.document.clone();
                let is_dirty_lock = self.is_dirty.clone();
                let selection = self.content.selection().clone();
                let id = self.id;

                if let Some(session_selection) = self.modal_content.session_selection {
                    if connection.is_some() && session_selection == SessionType::Read {
                        match action {
                            text_editor::Action::Edit(_) => {}
                            _ => self.content.perform(action.clone()),
                        }
                    } else {
                        self.content.perform(action.clone());
                    }
                } else {
                    self.content.perform(action.clone());
                }

                // Update markdown preview with the editor's text content
                self.markdown_text = markdown::parse(&self.content.text()).collect();

                let mut tasks = Vec::new();
                match action {
                    text_editor::Action::Edit(edit) => {
                        // Translate local user edit action to document operations
                        tasks.push(Task::future(async move {
                            let mut operations = Vec::new();
                            let mut doc = doc_lock.lock().await;
                            if let Some(id) = id {
                                doc.last_edit = id;
                            }

                            let num_deleted = if let Some(s) = selection {
                                // Find the selection in a slice of the content text
                                let start = if s.len() > index { 0 } else { index - s.len() };
                                let end = if index + s.len() > content_text.len() {
                                    content_text.len()
                                } else {
                                    index + s.len()
                                };

                                let text_to_search = content_text.get(start..end).unwrap_or("");
                                if let Some(i) = text_to_search.find(&s) {
                                    index = i + start;
                                    let deletion = doc.delete(index..(index + s.len()));
                                    operations.push(Operation::Delete(deletion));

                                    s.len()
                                } else {
                                    // Selection not found
                                    0
                                }
                            } else {
                                0
                            };

                            match edit {
                                text_editor::Edit::Insert(ch) => {
                                    let mut text = ch.to_string();

                                    if is_blank_line && !doc.check_newline_at(index) {
                                        text.push_str("\n"); // Insert newline after character
                                    }

                                    let insertion = doc.insert(index, text.clone());
                                    operations.push(Operation::Insert(insertion));
                                }
                                text_editor::Edit::Paste(text) => {
                                    let mut text = text.to_string();

                                    if is_blank_line && !doc.check_newline_at(index) {
                                        text.push_str("\n"); // Insert newline after string
                                    }

                                    let insertion = doc.insert(index, text);
                                    operations.push(Operation::Insert(insertion));
                                }
                                text_editor::Edit::Enter => {
                                    let text = String::from("\n");

                                    let insertion = doc.insert(index, text);
                                    operations.push(Operation::Insert(insertion));
                                }
                                text_editor::Edit::Delete => {
                                    if num_deleted == 0 && doc.len() > index + 1 {
                                        let deletion = doc.delete(index..(index + 1));
                                        operations.push(Operation::Delete(deletion));
                                    }

                                    if doc.len() == 1 {
                                        let deletion = doc.delete(0..1); // Remove remaining newline character
                                        operations.push(Operation::Delete(deletion));
                                    }
                                }
                                text_editor::Edit::Backspace => {
                                    if num_deleted == 0 && doc.len() > 1 {
                                        let deletion = doc.delete((index - 1)..index);
                                        operations.push(Operation::Delete(deletion));
                                    }

                                    if doc.len() == 1 {
                                        let deletion = doc.delete(0..1); // Remove remaining newline character
                                        operations.push(Operation::Delete(deletion));
                                    }
                                }
                            }

                            for op in operations.iter() {
                                if let Some(conn) = connection.as_mut() {
                                    println!("Sending edit request...");
                                    match op {
                                        Operation::Insert(insertion) => {
                                            conn.send(client::Message::User(format!(
                                                "Insert: {}",
                                                serde_json::to_string(insertion).unwrap()
                                            )));
                                        }
                                        Operation::Delete(deletion) => {
                                            conn.send(client::Message::User(format!(
                                                "Delete: {}",
                                                serde_json::to_string(deletion).unwrap()
                                            )));
                                        }
                                    }
                                }
                            }
                            if let Some(_) = connection {
                                operations.clear();
                            }
                            *is_dirty_lock.lock().await = true;

                            Message::NoOp
                        }));
                    }
                    text_editor::Action::Scroll { lines: _ } => return Task::done(Message::NoOp),
                    _ => tasks.push(Task::done(Message::NoOp)),
                }

                let line = self.cursor_position_in_pixels();
                self.cursor_marker.move_cursor(line);
                let cursor_marker = self.cursor_marker.clone();

                // Check if the user is connected to a session
                if let State::Connected(ref mut connection) = self.client_state {
                    if self.joined_session {
                        let cursor_data = serde_json::to_string(
                            &json!({ "y": line, "color": self.cursor_marker.color }),
                        )
                        .expect("Failed to serialize cursor data");
                        let message = format!("Cursor: {}", cursor_data);

                        // Send the message
                        connection.send(client::Message::User(message));
                    } else {
                        println!("Cannot send message; not joined in a session.");
                    }
                } else {
                    // If not, user is the host so update cursor position in the users map and user cursors list
                    let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
                    let users_lock = self.users.clone();
                    let is_moved_lock = self.is_moved.clone();
                    tasks.push(Task::future(async move {
                        let mut users = users_lock.lock().await;
                        users.add_user(localhost, Some(cursor_marker));
                        *is_moved_lock.lock().await = true;

                        let cursors = users.get_all_cursors();
                        Message::UpdateHostCursors(cursors)
                    }));
                }

                return Task::batch(tasks);
            }
            Message::Menu(menu_msg) => match menu_msg {
                MenuMessage::ThemeSelected(theme) => {
                    self.theme = theme;
                }
                MenuMessage::FileOpened(result) => match result {
                    Ok((path, contents)) => {
                        self.file = Some(path.clone());
                        self.content = text_editor::Content::with_text(&contents);
                        self.markdown_text = markdown::parse(&self.content.text()).collect();
                        println!("File loaded: {:?}", path);

                        let document = self.document.clone();
                        let content = self.content.text().clone();
                        return Task::future(async move {
                            let mut doc_lock = document.lock().await;
                            doc_lock.buffer = content;
                            Message::NoOp
                        });
                    }
                    Err(error) => {
                        println!("Failed to open file: {:?}", error);
                        // Handle error by showing an error message to the user or logging
                        self.content = text_editor::Content::with_text("Error loading file.");
                    }
                },
                MenuMessage::OpenFile => {
                    return Task::perform(open_file(), MenuMessage::FileOpened).map(Message::Menu);
                }
                MenuMessage::FileSaved(result) => match result {
                    Ok(path) => {
                        println!("File saved at: {}", path.display());
                    }
                    Err(error) => {
                        println!("Failed to save file: {:?}", error);
                        // Handle error by showing a failure message to the user
                        self.content = text_editor::Content::with_text("Error saving file.");
                    }
                },
                MenuMessage::SaveFile => {
                    return Task::perform(
                        save_file(self.file.clone(), self.content.text()),
                        MenuMessage::FileSaved,
                    )
                    .map(Message::Menu);
                }
            },
            Message::Format(text_style) => {
                let _ = self.format_bar.update(text_style.clone()); // Update the format bar UI

                return match text_style {
                    TextStyle::Bold => self.toggle_formatting(TextStyle::Bold),
                    TextStyle::Italic => self.toggle_formatting(TextStyle::Italic),
                    TextStyle::Strikethrough => self.toggle_formatting(TextStyle::Strikethrough),
                    TextStyle::TextSize(size) => {
                        // Update the text size
                        let text_size = if let Ok(size) = size.parse::<f32>() {
                            iced::Pixels::from(size)
                        } else {
                            iced::Pixels::from(DEFAULT_FONT_SIZE)
                        };

                        self.markdown_settings = markdown::Settings::with_text_size(text_size);
                        Task::done(Message::NoOp)
                    }
                };
            }
            Message::LinkClicked(url) => {
                println!("Link clicked: {}", url);
            }
            Message::NoOp => {}
            Message::DeleteLine => {
                let tasks = vec![
                    Task::done(Message::Action(text_editor::Action::SelectLine)),
                    Task::done(Message::Action(text_editor::Action::Edit(
                        text_editor::Edit::Delete,
                    ))),
                ];

                return Task::batch(tasks);
            }
            Message::DeleteWord => {
                let tasks = vec![
                    Task::done(Message::Action(text_editor::Action::SelectWord)),
                    Task::done(Message::Action(text_editor::Action::Edit(
                        text_editor::Edit::Delete,
                    ))),
                ];

                return Task::batch(tasks);
            }
            Message::ShortcutPaletteToggle => {
                self.shortcut_palette_open = !self.shortcut_palette_open;
            }
            Message::ShowMarkdownPreview(toggled) => {
                self.markdown_preview_open = toggled;
            }
            Message::StartSessionPressed => {
                // Verify that a server worker has been created
                if self.server_worker.is_none() {
                    return Task::done(Message::NoOp);
                }

                self.session_modal_open = !self.session_modal_open;
                self.started_session = true;

                let doc = self.document.clone();
                // If a file path is provided, load the file
                let file_path = self.modal_content.file_path_input.clone();
                let load_file_task = if !file_path.is_empty() {
                    Some(load_file(file_path.clone()))
                } else {
                    None
                };
                let is_dirty_lock = self.is_dirty.clone();
                let read_password = if self.modal_content.read_password_input.is_empty() {
                    self.read_password.clone()
                } else {
                    Some(self.modal_content.read_password_input.clone())
                };
                let edit_password = if self.modal_content.write_password_input.is_empty() {
                    self.edit_password.clone()
                } else {
                    Some(self.modal_content.write_password_input.clone())
                };
                let users_lock = self.users.clone();
                let is_moved_lock = self.is_moved.clone();
                let server_thread_lock = self.server_thread.clone();
                let server_worker = self.server_worker.clone().unwrap();
                self.id = Some(1);
                return Task::future(async move {
                    if let Some(load_task) = load_file_task {
                        match load_task.await {
                            Ok((_, contents)) => {
                                // Update the document with loaded file contents
                                let mut doc_lock = doc.lock().await;
                                doc_lock.buffer = contents.to_string();
                                // return Message::UpdateHostDoc(doc_lock.clone());
                            }
                            Err(err) => {
                                // Handle file load error (log or return an error message)
                                eprintln!("Failed to load file: {:?}", err);
                                return Message::NoOp;
                            }
                        }
                    }
                    let mut server_thread = server_thread_lock.lock().await;
                    users_lock.lock().await.add_user(
                        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080),
                        None,
                    );
                    *server_thread = Some(
                        start_server(
                            read_password,
                            edit_password,
                            doc.clone(),
                            is_dirty_lock,
                            users_lock,
                            is_moved_lock,
                            server_worker,
                        )
                        .await,
                    );
                    let dock_to_update = doc.lock().await.clone(); // Clone document for the update message
                    Message::UpdateHostDoc(dock_to_update)
                });
            }
            Message::UpdateHostDoc(document) => {
                // Update text editor content with the document content
                self.replace_content(&document);
            }
            Message::UpdateHostCursors(cursors) => {
                self.user_cursors = cursors;
            }
            Message::Echo(event) => match event {
                client::Event::ServerDown => {
                    self.joined_session = false;
                    self.modal_content.session_join_error =
                        "Server is down, please contact host.".to_string();
                }
                client::Event::IncorrectPassword => {
                    self.joined_session = false;
                    self.modal_content.session_join_error =
                        "Incorrect password, please try again.".to_string();
                }
                client::Event::Connected(connection) => {
                    self.client_state = State::Connected(connection.clone());
                    self.joined_session = true;
                    self.session_modal_open = false;

                    let line = self.cursor_position_in_pixels();

                    let cursor_data = serde_json::to_string(
                        &json!({ "y": line, "color": self.cursor_marker.color }),
                    )
                    .expect("Failed to serialize cursor data");
                    let message = format!("Cursor: {}", cursor_data);
                    if self.leave_session {
                        connection.clone().close();
                    }

                    // Send the message
                    connection.clone().send(client::Message::User(message));
                }
                client::Event::Disconnected => {
                    self.client_state = State::Disconnected;
                    println!("DISCONNECTED");
                    self.user_cursors.clear();
                }
                client::Event::MessageReceived(message) => {
                    // Extract the message as a string
                    let message_text = message.as_str();

                    let parts: Vec<&str> = message_text.split(":").collect();
                    let mut iter = parts.into_iter();
                    match iter.next() {
                        Some("Users") => {
                            // Extract the part of the message that represents users data
                            if let Some(users_start) = message_text.find("Users:") {
                                let users_data = &message_text[users_start + 6..]; // Skip "Users:"

                                // Attempt to parse the users data into a Users struct (you'll need to know how it's formatted)
                                if let Ok(users) = serde_json::from_str::<Users>(users_data.trim())
                                {
                                    // Clone the Arc<Mutex<Users>> for async access
                                    let users_lock = self.users.clone();
                                    self.user_cursors = users.get_all_cursors();
                                    // Update the mutex with the new users data
                                    return Task::future(async move {
                                        let mut locked_users = users_lock.lock().await;
                                        *locked_users = users;
                                        Message::NoOp
                                    });
                                } else {
                                    println!("Failed to parse users data");
                                }
                            }
                        }
                        Some("Document") => {
                            // Extract the part of the message that represents the document data
                            if let Some(document_start) = message_text.find("Document:") {
                                let document_data = &message_text[document_start + 9..]; // Skip "Document:"

                                // Update the document content in the editor
                                let server_doc =
                                    serde_json::from_str::<Document>(document_data.trim()).unwrap();

                                self.replace_content(&server_doc);

                                let doc_lock = self.document.clone();
                                return Task::future(async move {
                                    let mut doc = doc_lock.lock().await;
                                    *doc = server_doc;

                                    Message::NoOp
                                });
                            }
                        }
                        Some("Id") => {
                            // Extract the part of the message that represents the user's id
                            if let Some(id_start) = message_text.find("Id:") {
                                let id_data = &message_text[id_start + 3..]; // Skip "Id:"

                                // Update the user's id
                                let id = serde_json::from_str::<UserId>(id_data.trim()).unwrap();
                                self.id = Some(id);
                            }
                        }
                        _ => {}
                    }
                }
            },
            Message::ReadPasswordChanged(password) => {
                self.modal_content.read_password_input = password;
            }
            Message::WritePasswordChanged(password) => {
                self.modal_content.write_password_input = password;
            }
            Message::SessionPasswordChanged(password) => {
                self.modal_content.session_password_input = password;
            }
            Message::SessionTypeRequested(choice) => {
                self.modal_content.session_selection = Some(choice);
            }
            Message::JoinSessionPressed => {
                self.joined_session = true;
            }
            Message::SessionModalToggle => {
                self.session_modal_open = !self.session_modal_open;
            }
            Message::TabSelected(selected) => {
                self.active_tab = selected;
            }
            Message::FilePathChanged(file_path) => {
                self.modal_content.file_path_input = file_path;
                self.modal_content.validate_file();
            }
            Message::RequestClose => {
                println!("Closing server...");
                let server_thread_lock = self.server_thread.clone();
                let users_lock = self.users.clone();

                return Task::future(async move {
                    // Abort the server thread if it exists
                    let server_thread_mutex = server_thread_lock.lock().await;
                    if let Some(server_thread) = &*server_thread_mutex {
                        server_thread.abort();
                    }

                    // Clear all users
                    let mut users = users_lock.lock().await;
                    users.delete_all_users();

                    // Send the close window message
                    Message::SessionClosed
                });
            }
            Message::LeaveSession => {
                let connection = if let State::Connected(ref mut connection) = self.client_state {
                    Some(connection.clone())
                } else {
                    None
                };
                connection.unwrap().close();
                self.joined_session = false;
                self.client_state = State::Disconnected;
                self.user_cursors.clear();
                self.id = None;
            }
            Message::SessionClosed => {
                println!("Server closed");
                self.started_session = false;
                self.id = None;
            }
            Message::CloseWindow(id) => {
                println!("Window with id {:?} closed", id);
                return window::close::<iced::window::Id>(id).map(|_| Message::NoOp);
            }
            Message::WorkerReady(sender) => {
                self.server_worker = Some(sender);
            }
        }
        Task::none()
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn cursor_position_in_pixels(&self) -> f32 {
        let (line, _) = self.content.cursor_position();

        // Assuming you know font metrics
        let line_height = 21.0; // Adjust as per your font size

        line as f32 * line_height
    }

    fn replace_content(&mut self, doc: &Document) {
        if self.id == Some(doc.last_edit) {
            return;
        }

        let (line, col) = self.content.cursor_position();
        self.content = text_editor::Content::with_text(&doc.buffer);

        // Start at the beginning
        self.content.perform(text_editor::Action::Move(
            text_editor::Motion::DocumentStart,
        ));
        // Move to the right row
        (0..line).for_each(|_| {
            self.content
                .perform(text_editor::Action::Move(text_editor::Motion::Down));
        });

        // Scroll to the right col
        (0..col).for_each(|_| {
            self.content
                .perform(text_editor::Action::Move(text_editor::Motion::Right));
        });

        self.markdown_text = markdown::parse(&doc.buffer).collect();
    }

    fn toggle_formatting(&mut self, format: TextStyle) -> Task<Message> {
        let mut tasks = Vec::new();
        // Get the current selection in the editor, if any, and wrap it in the formatting symbol
        if let Some(selection) = self.content.selection() {
            // Check if the selection is already formatted in which case we remove the formatting
            let formatted_text = match format {
                TextStyle::Bold => {
                    if selection.starts_with("**") && selection.ends_with("**") {
                        selection
                            .strip_prefix("**")
                            .unwrap()
                            .strip_suffix("**")
                            .unwrap()
                            .to_string()
                    } else {
                        format!("**{}**", selection)
                    }
                }
                TextStyle::Italic => {
                    if (selection.starts_with("***") && selection.ends_with("***"))
                        || (!(selection.starts_with("**") && selection.ends_with("**"))
                            && selection.starts_with("*")
                            && selection.ends_with("*"))
                    {
                        selection
                            .strip_prefix("*")
                            .unwrap()
                            .strip_suffix("*")
                            .unwrap()
                            .to_string()
                    } else {
                        format!("*{}*", selection)
                    }
                }
                TextStyle::Strikethrough => {
                    if selection.starts_with("~~") && selection.ends_with("~~") {
                        selection
                            .strip_prefix("~~")
                            .unwrap()
                            .strip_suffix("~~")
                            .unwrap()
                            .to_string()
                    } else {
                        format!("~~{}~~", selection)
                    }
                }
                _ => {
                    return Task::done(Message::NoOp);
                }
            };

            // tasks.push(Task::done(Message::Action(text_editor::Action::Edit(
            //     text_editor::Edit::Delete,
            // ))));
            tasks.push(Task::done(Message::Action(text_editor::Action::Edit(
                text_editor::Edit::Paste(formatted_text.into()),
            ))));
        }

        tasks.push(Task::done(Message::Action(text_editor::Action::Move(
            text_editor::Motion::WordLeft,
        )))); // Move cursor to the right of the inserted text
        Task::batch(tasks)
    }
}

fn modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        Color {
                            a: 0.8,
                            ..Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
}

pub enum Input {
    Cursors(Vec<CursorMarker>),
    Edit(Document),
}

fn server_worker() -> impl Stream<Item = Message> {
    stream::channel(100, |mut output| async move {
        // Create channel
        let (sender, mut receiver) = mpsc::channel(100);

        // Send the sender back to the application
        output.send(Message::WorkerReady(sender)).await.unwrap();

        loop {
            use iced_futures::futures::StreamExt;

            // Read next input sent from `Application`
            let input = receiver.select_next_some().await;

            match input {
                Input::Cursors(cursors) => output
                    .send(Message::UpdateHostCursors(cursors))
                    .await
                    .unwrap(),
                Input::Edit(document) => {
                    output.send(Message::UpdateHostDoc(document)).await.unwrap()
                }
            }
        }
    })
}
