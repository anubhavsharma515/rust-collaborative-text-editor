use iced::widget::{column, pick_list};
use iced::{Alignment, Task, Element, Length, Theme};

#[derive(Debug, Clone)]
pub enum MenuMessage {
    ThemeSelected(Theme),
}

pub struct MenuBar {
    selected_theme: Theme,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            selected_theme: Theme::Light,
        }
    }

    pub fn update(&mut self, message: MenuMessage) -> Task<MenuMessage> {
        match message {
            MenuMessage::ThemeSelected(theme) => {
                self.selected_theme = theme.clone();
                println!("Theme selected: {:?}", theme);
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

        column![
            theme_selector,
        ]
        .spacing(20)
        .align_x(Alignment::Center)
        .padding(10)
        .into()
    }
}
