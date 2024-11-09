use iced::widget::{pick_list, button, row};
use iced::{Alignment, Task, Element, Length, Theme};

#[derive(Debug, Clone)]
pub enum MenuMessage {
    ThemeSelected(Theme),
    ToggleBold,
    ToggleItalic,
}

pub struct MenuBar {
    selected_theme: Theme,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            selected_theme: Theme::default(),
        }
    }

    pub fn update(&mut self, message: MenuMessage) -> Task<MenuMessage> {
        match message {
            MenuMessage::ThemeSelected(theme) => {
                self.selected_theme = theme.clone();
                println!("Theme selected: {:?}", theme);
            },
            MenuMessage::ToggleBold => {
                println!("Bold toggled");
            },
            MenuMessage::ToggleItalic => {
                println!("Italic toggled");
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<MenuMessage> {
        let theme_selector = pick_list(
            Theme::ALL,
            Some(&self.selected_theme),
            MenuMessage::ThemeSelected,
        )
        .width(Length::Shrink)
        .padding(5);

        let bold_button = button("Bold")
            .on_press(MenuMessage::ToggleBold)
            .padding(5);

        let italic_button = button("Italic")
            .on_press(MenuMessage::ToggleItalic)
            .padding(5);

        row![
            theme_selector,
            bold_button,
            italic_button
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }
}
