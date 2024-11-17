use iced::widget::{button, row, text, text_input};
use iced::{Alignment, Element, Length, Task};

pub const DEFAULT_TEXT_SIZE: u16 = 16;

#[derive(Debug, Clone)]
pub enum TextStyle {
    Bold,
    Italic,
    Strikethrough,
    TextSize(String),
}

pub struct FormatBar {
    text_size: String,
}

impl FormatBar {
    pub fn new() -> Self {
        Self {
            text_size: DEFAULT_TEXT_SIZE.to_string(),
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
            TextStyle::TextSize(text_size) => {
                self.text_size = text_size;
            }
            _ => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<TextStyle> {
        let text_size = &self.text_size.to_string();

        let text_size_input = text_input("16", text_size)
            .on_input(TextStyle::TextSize)
            .width(Length::Fixed(45.0))
            .line_height(text::LineHeight::Relative(1.0))
            .padding(10)
            .size(16);

        let bold_button = button("Bold").on_press(TextStyle::Bold).padding(5);

        let italic_button = button("Italic").on_press(TextStyle::Italic).padding(5);

        let strikethrough_button = button("Strikethrough")
            .on_press(TextStyle::Strikethrough)
            .padding(5);

        row![
            bold_button,
            italic_button,
            strikethrough_button,
            text_size_input
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }
}
