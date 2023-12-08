#![windows_subsystem = "windows"]

use iced::{Application, Settings};
mod server;
mod window;
use clap::Parser;
use window::Gui;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(author = "Radical <Radiicall> <radical@radical.fun>")]
#[command(version)]
#[command(about = "Rich presence for Jellyfin", long_about = None)]
pub struct Args {
    #[arg(short = 'c', long = "config", help = "Path to the config file")]
    pub config: Option<String>,
    #[arg(
        short = 'i',
        long = "image-urls-file",
        help = "Path to image urls file for imgur"
    )]
    pub image_urls: Option<String>,
}

#[tokio::main()]
pub async fn main() -> iced::Result {
    Gui::run(Settings {
        window: iced::window::Settings {
            size: (350, 500),
            resizable: false,
            ..Default::default()
        },
        ..Default::default()
    })
}
