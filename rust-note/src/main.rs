use iced::widget::{column, text_editor};
use iced::{Task, Element};

#[derive(Default)]
struct Editor {
    content: text_editor::Content,
}

pub fn main() -> iced::Result {
    iced::application(
        Editor::title,
        Editor::update,
        Editor::view
    ).run_with(Editor::new)
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
}

impl Editor {

    fn new() -> (Self, Task<Message>) {
        ( 
            Self { 
                content: text_editor::Content::new()
            },
            Task::none()
        )
    }

    fn title(&self) -> String {
        String::from("rust-note")
    }

    fn view(&self) -> Element<Message> {
        column![
            text_editor(&self.content)
                .on_action(Message::Edit)
        ]
        .into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {

                self.content.perform(action);

                Task::none()
            }
        }

    }
}

