// Custom widgets
mod editor;
mod handlers;
mod server;
mod widgets;

use editor::Editor;
use server::start_server;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    host_file: bool,

    #[structopt(short, long)]
    read_access_password: Option<String>,

    #[structopt(short, long)]
    write_access_password: Option<String>,
}

#[tokio::main]
pub async fn main() -> iced::Result {
    let opt = Opt::from_args();
    let ct_txt_1 = Arc::new(Mutex::new(String::new()));
    let ct_txt_2 = Arc::clone(&ct_txt_1);

    let server_thread = if opt.host_file {
        Some(
            start_server(
                opt.read_access_password,
                opt.write_access_password,
                ct_txt_1,
            )
            .await,
        )
    } else {
        None
    };

    iced::application(Editor::title, Editor::update, Editor::view)
        .font(include_bytes!("../fonts/format-bar-icons.ttf").as_slice())
        .theme(Editor::theme)
        .exit_on_close_request(false)
        .subscription(Editor::subscription)
        .run_with(move || Editor::new(ct_txt_2, server_thread))
}
