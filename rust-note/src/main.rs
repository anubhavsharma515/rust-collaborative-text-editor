use std::ffi;
use std::path::{Path, PathBuf};

use iced::highlighter;
use iced::widget::{column, horizontal_space, markdown, row, scrollable, text, text_editor};
use iced::keyboard;
use iced::{Alignment, Element, Length, Task, Theme};

// Custom widgets
mod widgets;

use widgets::format_bar::{FormatBar, TextStyle, DEFAULT_TEXT_SIZE};
use widgets::menubar::{MenuBar, MenuMessage, open_file};

const BOLD_HOTKEY: &str = "b";
const ITALIC_HOTKEY: &str = "i";
const STRIKETHROUGH_HOTKEY: &str = "f";

pub struct Editor {
    content: text_editor::Content,
    menubar: MenuBar,
    format_bar: FormatBar,
    file: Option<PathBuf>,
    theme: Theme,
    markdown_text: Vec<markdown::Item>,
    markdown_settings: markdown::Settings,
}

pub fn main() -> iced::Result {
    iced::application(Editor::title, Editor::update, Editor::view)
        .theme(Editor::theme)
        .run_with(Editor::new)
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    Menu(MenuMessage),
    Format(TextStyle),
    LinkClicked(markdown::Url),
}

impl Editor {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                menubar: MenuBar::new(),
                format_bar: FormatBar::new(),
                file: None,
                theme: Theme::default(),
                markdown_text: markdown::parse("Write your **Markdown** text here.").collect(),
                markdown_settings: markdown::Settings::with_text_size(DEFAULT_TEXT_SIZE),
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        String::from("rust-note")
    }

    fn view(&self) -> Element<'_, Message> {
        let mut markdown_settings = markdown::Settings::default();
        markdown_settings.text_size = iced::Pixels(50.0);

        // Create the status bar
        let status = row![
            text(if let Some(path) = &self.file {
                let path = path.display().to_string();

                if path.len() > 60 {
                    format!("...{}", &path[path.len() - 40..])
                } else {
                    path
                }
            } else {
                String::from("New file")
            }),
            horizontal_space(),
            text({
                let (line, column) = self.content.cursor_position();

                format!("{}:{}", line + 1, column + 1)
            })
        ]
        .spacing(10);

        column![
            self.menubar.view().map(Message::Menu),
            self.format_bar.view().map(Message::Format),
            row![
                text_editor(&self.content)
                    .highlight(
                        self.file
                            .as_deref()
                            .and_then(Path::extension)
                            .and_then(ffi::OsStr::to_str)
                            .unwrap_or("rs"),
                        highlighter::Theme::SolarizedDark,
                    )
                    .height(Length::FillPortion(1))
                    .on_action(Message::Edit)
                    .key_binding(|key_press| {
                        match key_press.key.as_ref() {
                            keyboard::Key::Character(BOLD_HOTKEY)
                                if key_press.modifiers.command() =>
                            {
                                Some(text_editor::Binding::Custom(Message::Format(
                                    TextStyle::Bold,
                                )))
                            }
                            keyboard::Key::Character(ITALIC_HOTKEY)
                                if key_press.modifiers.command() =>
                            {
                                Some(text_editor::Binding::Custom(Message::Format(
                                    TextStyle::Italic,
                                )))
                            }
                            keyboard::Key::Character(STRIKETHROUGH_HOTKEY)
                                if key_press.modifiers.command() =>
                            {
                                Some(text_editor::Binding::Custom(Message::Format(
                                    TextStyle::Strikethrough,
                                )))
                            }
                            _ => text_editor::Binding::from_key_press(key_press),
                        }
                    }),
                scrollable(
                markdown::view(
                    &self.markdown_text,
                    self.markdown_settings,
                    markdown::Style::from_palette(self.theme.clone().palette()),
                )
                .map(Message::LinkClicked),
                )
            ]
            .spacing(20)
            .align_y(Alignment::Start),
            status, // Add the status widget here
        ]
        .align_x(Alignment::Center)
        .spacing(10)
        .into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);

                // Update markdown preview with the editor's text content
                self.markdown_text = markdown::parse(&self.content.text()).collect()
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
                        return Task::perform(open_file(), MenuMessage::FileOpened).map(Message::Menu);
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
                            iced::Pixels::from(DEFAULT_TEXT_SIZE)
                        };

                        self.markdown_settings = markdown::Settings::with_text_size(text_size);
                    }
                    _ => {}
                }
            }
            Message::LinkClicked(url) => {
                println!("Link clicked: {}", url);
            }
        }
        Task::none()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
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
