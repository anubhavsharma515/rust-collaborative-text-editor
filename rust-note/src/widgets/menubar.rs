use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{button, column, pick_list, row, text_editor};
use iced::{Alignment, Element, Length, Task, Theme};

#[derive(Debug, Clone)]
pub enum MenuMessage {
    ThemeSelected(Theme),
    OpenFile,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    SaveFile,
    FileSaved(Result<PathBuf, Error>),
}

pub struct MenuBar {
    selected_theme: Theme,
    file: Option<PathBuf>,
    content: Arc<String>,
    is_loading: bool,
    is_dirty: bool,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            selected_theme: Theme::default(),
            file: None,
            content: Arc::new(String::new()),
            is_loading: true,
            is_dirty: false,
        }
    }

    pub fn update(&mut self, message: MenuMessage) -> Task<MenuMessage> {
        match message {
            MenuMessage::ThemeSelected(theme) => {
                self.selected_theme = theme.clone();
                println!("Theme selected: {:?}", theme);

                Task::none()
            }
            MenuMessage::OpenFile => {
                if self.is_loading {
                    Task::none()
                } else {
                    self.is_loading = true;
                    Task::perform(open_file(), MenuMessage::FileOpened)
                }
            }
            MenuMessage::FileOpened(result) => {
                self.is_loading = false;
                self.is_dirty = false;

                if let Ok((path, contents)) = result {
                    self.file = Some(path);
                    self.content = contents;
                }

                Task::none()
            }
            MenuMessage::SaveFile => Task::none(),
            MenuMessage::FileSaved(_) => Task::none(),
        }
    }

    pub fn view(&self) -> Element<MenuMessage> {
        let file_picker = button("Open File")
            .on_press(MenuMessage::OpenFile)
            .padding(5);
        let file_save = button("Save File")
            .on_press(MenuMessage::SaveFile)
            .padding(5);

        let theme_selector = pick_list(
            Theme::ALL,
            Some(&self.selected_theme),
            MenuMessage::ThemeSelected,
        )
        .width(Length::Shrink)
        .padding(5);

        row![file_picker, file_save, theme_selector]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    DialogClosed,
    IoError(io::ErrorKind),
}

pub async fn open_file() -> Result<(PathBuf, Arc<String>), Error> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Open a text file...")
        .add_filter("Text Files", &["md"])
        .pick_file()
        .await
        .ok_or(Error::DialogClosed)?;

    load_file(picked_file).await
}

pub async fn load_file(path: impl Into<PathBuf>) -> Result<(PathBuf, Arc<String>), Error> {
    let path = path.into();

    let contents = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|error| Error::IoError(error.kind()))?;

    Ok((path, contents))
}

pub async fn save_file(path: Option<PathBuf>, contents: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .save_file()
            .await
            .as_ref()
            .map(rfd::FileHandle::path)
            .map(Path::to_owned)
            .ok_or(Error::DialogClosed)?
    };

    tokio::fs::write(&path, contents)
        .await
        .map_err(|error| Error::IoError(error.kind()))?;

    Ok(path)
}
