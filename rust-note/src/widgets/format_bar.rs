use iced::widget::{button, row};
use iced::{Alignment, Element, Pixels, Task};

#[derive(Debug, Clone)]
pub enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
}

pub struct FormatBar {
    font_size: Pixels,
}

impl FormatBar {
    pub fn new() -> Self {
        Self {
            font_size: Pixels(16.0),
        }
    }

    pub fn update(&mut self, message: TextStyle) -> Task<TextStyle> {
        match message {
            TextStyle::Bold => {
                println!("Bold toggled");
            }
            TextStyle::Italic => {
                println!("Italic toggled");
            }
            _ => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<TextStyle> {
        let bold_button = button("Bold").on_press(TextStyle::Bold).padding(5);

        let italic_button = button("Italic").on_press(TextStyle::Italic).padding(5);

        let strikethrough_button = button("Strikethrough")
            .on_press(TextStyle::Strikethrough)
            .padding(5);

        row![bold_button, italic_button, strikethrough_button]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
    }
}
