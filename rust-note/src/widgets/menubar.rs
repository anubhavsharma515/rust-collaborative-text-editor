use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{button, pick_list, row};
use iced::{Alignment, Element, Length, Theme};

#[derive(Debug, Clone)]
pub enum MenuMessage {
    ThemeSelected(Theme),
    OpenFile,
    FileOpened(Result<(PathBuf, Arc<String>), String>),
    SaveFile,
    CloseFile,
    FileSaved(Result<PathBuf, String>),
}

pub struct MenuBar;

impl MenuBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn view(
        &self,
        theme: Theme,
        disable_open_file: bool,
        file_opened: bool,
    ) -> Element<'_, MenuMessage> {
        let file_picker = if disable_open_file {
            button("Open File").padding(5)
        } else {
            button("Open File")
                .on_press(MenuMessage::OpenFile)
                .padding(5)
        };
        let file_save = button("Save File")
            .on_press(MenuMessage::SaveFile)
            .padding(5);

        let file_close = if file_opened {
            button("Close File").padding(5)
        } else {
            button("Close File")
                .on_press(MenuMessage::CloseFile)
                .padding(5)
        };

        let theme_selector = pick_list(Theme::ALL, Some(theme), MenuMessage::ThemeSelected)
            .width(Length::Shrink)
            .padding(5);

        row![file_picker, file_save, file_close, theme_selector]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
    }
}

pub async fn open_file() -> Result<(PathBuf, Arc<String>), String> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Open a text file...")
        .add_filter("Text Files", &["md", "txt"])
        .pick_file()
        .await
        .ok_or_else(|| "File dialog closed without selection.".to_string())?;

    load_file(picked_file)
        .await
        .map_err(|_| "File dialog was closed.".to_string())
}

pub async fn load_file(path: impl Into<PathBuf>) -> Result<(PathBuf, Arc<String>), String> {
    let path = path.into();

    let contents = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|err| format!("IO error reading file: {}", err))?; // Convert error to a simple string

    println!("File loaded successfully from: {}", path.display()); // Log successful load
    Ok((path, contents))
}

pub async fn save_file(path: Option<PathBuf>, contents: String) -> Result<PathBuf, String> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .save_file()
            .await
            .as_ref()
            .map(rfd::FileHandle::path)
            .map(Path::to_owned)
            .ok_or_else(|| "Save file dialog was closed without selection.".to_string())?
    };

    tokio::fs::write(&path, contents)
        .await
        .map_err(|err| format!("Failed to write file: {}", err))?; // Convert error to a simple string

    println!("File saved successfully at: {}", path.display()); // Log successful save
    Ok(path)
}
