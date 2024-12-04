use crate::server::start_server;
use crate::server::UserCursorPositions;
use crate::widgets;
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Path as icedPath};
use iced::widget::{
    button, center, column, container, horizontal_space, markdown, mouse_area, opaque, row,
    scrollable, stack, text, text_editor, text_input, toggler, Container, Stack, Text, TextEditor,
};
use iced::Subscription;
use iced::{highlighter, Color};
use iced::{window, Pixels};
use iced::{Alignment, Element, Length, Task, Theme};
use iced::{Point, Rectangle, Renderer, Size};
use iced_aw::{TabLabel, Tabs};
use serde::Deserialize;
use serde::Serialize;
use std::ffi;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use widgets::format_bar::{FormatBar, TextStyle, DEFAULT_FONT_SIZE};
use widgets::menubar::{open_file, save_file, MenuBar, MenuMessage};

const BOLD_HOTKEY: &str = "b";
const ITALIC_HOTKEY: &str = "i";
const STRIKETHROUGH_HOTKEY: &str = "f";
const SHORTCUT_PALETTE_HOTKEY: &str = "p";
const SESSION_MODAL_HOTKEY: &str = "n";

pub struct SessionModal {
    pub name_input: String,
    pub server_input: String,
}

impl Default for SessionModal {
    fn default() -> Self {
        Self {
            name_input: String::new(),
            server_input: String::new(),
        }
    }
}

pub struct Editor {
    content: text_editor::Content,
    content_text: Arc<Mutex<String>>,
    cursor_marker: CursorMarker,
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
    server_thread: Option<JoinHandle<()>>,
    users: Arc<Mutex<UserCursorPositions>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Edit(text_editor::Action),
    Menu(MenuMessage),
    Format(TextStyle),
    LinkClicked(markdown::Url),
    ShowMarkdownPreview(bool),
    NoOp,
    DeleteLine,
    DeleteWord,
    ShortcutPaletteToggle,
    SessionModalToggle,
    LoginNameChanged(String),
    LoginServerChanged(String),
    LoginButtonPressed,
    TabSelected(TabId),
    CloseWindow(iced::window::Id),
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum TabId {
    #[default]
    StartSession,
    JoinSession,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct CursorMarker {
    pub y: f32,
}

impl CursorMarker {
    pub fn new(y: f32) -> Self {
        Self { y }
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
        frame.fill(&rectangle, Color::from_rgb(0.0, 0.8, 0.2));
        vec![frame.into_geometry()]
    }
}

impl Editor {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                content_text: Arc::new(Mutex::new(String::new())),
                cursor_marker: CursorMarker::new(0.2),
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
                server_thread: None,
                users: Arc::new(Mutex::new(UserCursorPositions::new())),
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        String::from("rust-note")
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let subscriptions = vec![window::events().map(|(id, evt)| match evt {
            iced::window::Event::CloseRequested => Message::CloseWindow(id),
            _ => Message::NoOp,
        })];

        Subscription::batch(subscriptions)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut markdown_settings = markdown::Settings::default();
        markdown_settings.text_size = iced::Pixels(50.0);

        let status = row![
            button("Collaborate")
                .on_press(Message::SessionModalToggle)
                .style(button::primary),
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
                            text_input("Enter your name", &self.modal_content.name_input)
                                .on_input(Message::LoginNameChanged)
                                .padding(5),
                            text_input("Enter server address", &self.modal_content.server_input)
                                .on_input(Message::LoginServerChanged)
                                .padding(5),
                            {
                                let mut button = button("Start Session").style(button::secondary);
                                if !(self.modal_content.name_input.is_empty()
                                    | self.modal_content.server_input.is_empty())
                                {
                                    button = button
                                        .on_press(Message::LoginButtonPressed)
                                        .style(button::primary)
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
                            text_input("Enter server address", &self.modal_content.server_input)
                                .on_input(Message::LoginServerChanged)
                                .padding(5),
                            {
                                let mut button = button("Start Session").style(button::secondary);
                                if !(self.modal_content.server_input.is_empty()) {
                                    button = button
                                        .on_press(Message::LoginButtonPressed)
                                        .style(button::primary)
                                }
                                button
                            }
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
        .height(Length::Fixed(300.0))
        .width(300)
        .padding(10)
        .style(container::rounded_box);

        let marker: Canvas<CursorMarker, Message> = Canvas::new(self.cursor_marker.clone())
            .width(Length::FillPortion(1))
            .height(Length::Fill);

        let editor = TextEditor::new(&self.content)
            .line_height(text::LineHeight::Absolute(Pixels(21.0)))
            .highlight(
                self.file
                    .as_deref()
                    .and_then(Path::extension)
                    .and_then(ffi::OsStr::to_str)
                    .unwrap_or("txt"),
                highlighter::Theme::SolarizedDark,
            )
            .wrapping(text::Wrapping::WordOrGlyph)
            .width(300)
            .height(Length::FillPortion(1))
            .on_action(Message::Edit)
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

        let content = column![
            row![
                self.menubar.view().map(Message::Menu),
                toggler(self.markdown_preview_open)
                    .label("Show Markdown preview")
                    .on_toggle(Message::ShowMarkdownPreview)
            ]
            .spacing(15),
            self.format_bar.view().map(Message::Format),
            row![
                Stack::with_children(vec![editor.into(), marker.into()])
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
            Message::Edit(action) => {
                self.content.perform(action);
                let line = self.cursor_position_in_pixels();
                self.cursor_marker = CursorMarker::new(line);

                // Update markdown preview with the editor's text content
                self.markdown_text = markdown::parse(&self.content.text()).collect();

                // Update the shared content text
                let text = self.content.text();
                let content_lock = self.content_text.clone();
                return Task::future(async move {
                    let mut content_txt = content_lock.lock().await;
                    *content_txt = text;
                    Message::NoOp
                });
            }
            Message::Menu(menu_msg) => {
                let _ = self.menubar.update(menu_msg.clone());

                match menu_msg {
                    MenuMessage::ThemeSelected(theme) => {
                        self.theme = theme;
                    }
                    MenuMessage::FileOpened(result) => {
                        if let Ok((path, contents)) = result {
                            self.file = Some(path.clone());
                            self.content = text_editor::Content::with_text(&contents);
                            self.markdown_text = markdown::parse(&self.content.text()).collect();
                            println!("File loaded: {:?}", path);
                        }
                    }
                    MenuMessage::OpenFile => {
                        return Task::perform(open_file(), MenuMessage::FileOpened)
                            .map(Message::Menu);
                    }
                    MenuMessage::FileSaved(_) => {}
                    MenuMessage::SaveFile => {
                        return Task::perform(
                            save_file(self.file.clone(), self.content.text()),
                            MenuMessage::FileSaved,
                        )
                        .map(|_| Message::NoOp);
                    }
                }
            }
            Message::Format(text_style) => {
                let _ = self.format_bar.update(text_style.clone()); // Update the format bar UI

                match text_style {
                    TextStyle::Bold => {
                        self.toggle_formatting(TextStyle::Bold);
                    }
                    TextStyle::Italic => {
                        self.toggle_formatting(TextStyle::Italic);
                    }
                    TextStyle::Strikethrough => {
                        self.toggle_formatting(TextStyle::Strikethrough);
                    }
                    TextStyle::TextSize(size) => {
                        // Update the text size
                        let text_size = if let Ok(size) = size.parse::<f32>() {
                            iced::Pixels::from(size)
                        } else {
                            iced::Pixels::from(DEFAULT_FONT_SIZE)
                        };

                        self.markdown_settings = markdown::Settings::with_text_size(text_size);
                    }
                }
            }
            Message::LinkClicked(url) => {
                println!("Link clicked: {}", url);
            }
            Message::NoOp => {}
            Message::DeleteLine => {
                self.content.perform(text_editor::Action::SelectLine);
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
            }
            Message::DeleteWord => {
                self.content.perform(text_editor::Action::SelectWord);
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
            }
            Message::ShortcutPaletteToggle => {
                self.shortcut_palette_open = !self.shortcut_palette_open;
            }
            Message::ShowMarkdownPreview(toggled) => {
                self.markdown_preview_open = toggled;
            }
            Message::LoginNameChanged(name) => {
                self.modal_content.name_input = name;
            }
            Message::LoginServerChanged(server) => {
                self.modal_content.server_input = server;
            }
            Message::LoginButtonPressed => {
                let content_text = self.content_text.clone();
                let user_to_cursor_map = self.users.clone();
                return Task::future(async move {
                    start_server(None, None, content_text, user_to_cursor_map).await;
                    // TODO: Figure out a way to pass the handle to self.server_thread
                    Message::NoOp
                });
            }
            Message::SessionModalToggle => {
                self.session_modal_open = !self.session_modal_open;
                self.modal_content.name_input.clear();
                self.modal_content.server_input.clear();
            }
            Message::TabSelected(selected) => {
                self.active_tab = selected;
                if !(self.modal_content.server_input.is_empty()) {
                    self.modal_content.server_input.clear();
                }
            }
            Message::CloseWindow(id) => {
                println!("Window with id {:?} closed", id);
                println!("Closing server...");
                if let Some(server_thread) = &self.server_thread {
                    server_thread.abort();
                }

                return window::close(id);
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

    fn toggle_formatting(&mut self, format: TextStyle) {
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
                    return;
                }
            };

            self.content
                .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
            self.content
                .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                    formatted_text.into(),
                )));
        }
        self.content
            .perform(text_editor::Action::Move(text_editor::Motion::WordLeft)); // Move cursor to the right of the inserted text
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
