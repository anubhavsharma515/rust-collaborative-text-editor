use iced::widget::{button, container, row, text, text_input, tooltip};
use iced::{Alignment, Element, Font, Length, Task};

pub const DEFAULT_FONT_SIZE: u16 = 16;

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
            text_size: DEFAULT_FONT_SIZE.to_string(),
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

        let bold_button = format_bar_button(bold_icon(), "Bold", TextStyle::Bold);
        let italic_button = format_bar_button(italic_icon(), "Italic", TextStyle::Italic);
        let strikethrough_button = format_bar_button(
            strikethrough_icon(),
            "Strikethrough",
            TextStyle::Strikethrough,
        );

        row![
            bold_button,
            italic_button,
            strikethrough_button,
            container(text_size_icon(20))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            text_size_input
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }
}

fn format_bar_button<'a>(
    content: Element<'a, TextStyle>,
    label: &'a str,
    on_press: TextStyle,
) -> Element<'a, TextStyle> {
    tooltip(
        button(container(content).width(30).align_x(Alignment::Center))
            .on_press(on_press)
            .padding(5),
        label,
        tooltip::Position::Bottom,
    )
    .into()
}

fn bold_icon<'a>() -> Element<'a, TextStyle> {
    icon('\u{E800}', None)
}

fn italic_icon<'a>() -> Element<'a, TextStyle> {
    icon('\u{E801}', None)
}

fn strikethrough_icon<'a>() -> Element<'a, TextStyle> {
    icon('\u{F0CC}', None)
}

fn text_size_icon<'a>(font_size: u16) -> Element<'a, TextStyle> {
    icon('\u{F088}', Some(font_size))
}

fn icon<'a>(unicode: char, font_size: Option<u16>) -> Element<'a, TextStyle> {
    const ICON_FONT: Font = Font::with_name("format-bar-icons");

    text(unicode)
        .font(ICON_FONT)
        .size(font_size.unwrap_or_else(|| DEFAULT_FONT_SIZE))
        .into()
}
