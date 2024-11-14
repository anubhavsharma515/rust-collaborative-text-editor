use iced::widget::{column, markdown, row, text_editor};
use iced::{Alignment, Element, Length, Task, Theme};

// Custom widgets
mod widgets;

use widgets::format_bar::{FormatBar, TextStyle};
use widgets::menubar::{MenuBar, MenuMessage};

pub struct Editor {
    content: text_editor::Content,
    menubar: MenuBar,
    format_bar: FormatBar,
    theme: Theme,
    markdown_text: Vec<markdown::Item>,
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
                theme: Theme::default(),
                markdown_text: markdown::parse("Write your **Markdown** text here.").collect(),
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
        column![
            self.menubar.view().map(Message::Menu),
            self.format_bar.view().map(Message::Format),
            row![
                text_editor(&self.content)
                    .height(Length::FillPortion(1))
                    .on_action(Message::Edit),
                markdown::view(
                    &self.markdown_text,
                    markdown_settings,
                    markdown::Style::from_palette(Theme::TokyoNightStorm.palette()),
                )
                .map(Message::LinkClicked)
            ]
            .spacing(20)
            .align_y(Alignment::Center)
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
                }
            }
            Message::Format(text_style) => {
                // Apply functionality first, then update the UI
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
                }

                let _ = self.format_bar.update(text_style); // Update the format bar UI
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
            };

            self.content
                .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
            self.content
                .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                    formatted_text.into(),
                )));
        }
        // self.content.perform(text_editor::Action::Move(text_editor::Motion::WordRight)); // Move cursor to the right of the inserted text
    }
}
