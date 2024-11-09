use iced::widget::{column, markdown, text_editor, row, scrollable};
use iced::{Alignment, Task, Element, Fill, Theme, Length};

// Custom widgets
mod widgets;

use widgets::menubar::{MenuBar, MenuMessage};

pub struct Editor {
    content: text_editor::Content,
    menubar: MenuBar,
    theme: Theme,
    markdown_text: String,
    is_bold: bool,
    is_italic: bool,
}

pub fn main() -> iced::Result {
    iced::application(
        Editor::title,
        Editor::update,
        Editor::view
    )
        .theme(Editor::theme) 
        .run_with(Editor::new)
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    Menu(MenuMessage),
}

impl Editor {

    fn new() -> (Self, Task<Message>) {
        ( 
            Self {
                content: text_editor::Content::new(),
                menubar: MenuBar::new(),
                theme: Theme::default(),
                markdown_text: String::from("Write your **Markdown** text here."),
                is_bold: false,
                is_italic: false,
            },
            Task::none()
        )
    }

    fn title(&self) -> String {
        String::from("rust-note")
    }

    fn view(&self) -> Element<Message> {
        column![
            self.menubar.view().map(Message::Menu),
            row![
                text_editor(&self.content)
                    .height(Length::FillPortion(1))
                    .on_action(Message::Edit),
                // markdown::view(
                //     markdown::parse(&self.markdown_text).collect(), 
                //     markdown::Settings::default(), 
                //     markdown::Style::from_palette(Theme::TokyoNightStorm.palette()),
                // )
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
                // self.markdown_text = self.content.value().to_string();
            },
            Message::Menu(menu_msg) => {
                self.menubar.update(menu_msg.clone());

                match menu_msg {
                    MenuMessage::ThemeSelected(theme) => {
                        self.theme = theme;
                    }
                    MenuMessage::ToggleBold => {
                        self.toggle_formatting("**");
                    }
                    MenuMessage::ToggleItalic => {
                        self.toggle_formatting("*");
                    }
                }
            }
        }
        Task::none()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    } 

    fn toggle_formatting(&mut self, symbol: &str) {
        // Get the current selection in the editor, if any, and wrap it in the formatting symbol
        let text = self.content.text();
        let selected_text = self.content.selection().unwrap();
        println!("{text}");
        println!("{symbol}{selected_text}{symbol}");
    }
}

