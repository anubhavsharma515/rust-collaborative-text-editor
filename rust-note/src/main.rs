// Custom widgets
mod client;
mod editor;
mod handlers;
mod server;
mod widgets;

use editor::Editor;

#[tokio::main]
pub async fn main() -> iced::Result {
    iced::application(Editor::title, Editor::update, Editor::view)
        .font(include_bytes!("../fonts/format-bar-icons.ttf").as_slice())
        .theme(Editor::theme)
        .exit_on_close_request(false)
        .subscription(Editor::subscription)
        .run_with(move || Editor::new())
}
