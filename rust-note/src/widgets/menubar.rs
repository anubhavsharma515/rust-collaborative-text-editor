use iced::widget::{pick_list, row};
use iced::{Alignment, Element, Length, Task, Theme};

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
            selected_theme: Theme::default(),
        }
    }

    pub fn update(&mut self, message: MenuMessage) -> Task<MenuMessage> {
        match message {
            MenuMessage::ThemeSelected(theme) => {
                self.selected_theme = theme.clone();
                println!("Theme selected: {:?}", theme);
            }
            _ => {}
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

        row![theme_selector]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
    }
}
