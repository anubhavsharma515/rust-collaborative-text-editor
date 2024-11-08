use iced::widget::{column, text_editor};
use iced::{Task, Element, Theme};

// Custom widgets
mod widgets;

use widgets::menubar::{MenuBar, MenuMessage};

pub struct Editor {
    content: text_editor::Content,
    menubar: MenuBar,
    theme: Theme,
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
                theme: Theme::Light,
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
            text_editor(&self.content)
                .on_action(Message::Edit)
        ]
        .into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
            },
            Message::Menu(menu_msg) => {
                self.menubar.update(menu_msg.clone());
                // Update the selected theme when the menu item is selected
                if let MenuMessage::ThemeSelected(theme) = menu_msg {
                    self.theme = theme; // Update the theme
                }
            },
        }
        Task::none()
    } 

    fn theme(&self) -> Theme {
        self.theme.clone()
    }
}

