use crate::VERSION;
use discord_rich_presence::DiscordIpcClient;
use jellyfin_rpc::{prelude::*, core::config::Username};
use std::sync::mpsc;

pub async fn run(
    config_path: String,
    mut config: Config,
    image_urls: Option<String>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Command>,
) {
    tokio::spawn(async move {
        if config.jellyfin.api_key.is_empty() {
            tx.send(Event::Error("Jellyfin API key not set".to_string(), "".to_string()))
                .unwrap();
            return;
        } else if config.jellyfin.url.is_empty() {
            tx.send(Event::Error("Jellyfin URL not set".to_string(), "".to_string()))
                .unwrap();
            return;
        } else if config.jellyfin.username == Username::String("".to_string()) {
            tx.send(Event::Error("Jellyfin Username not set".to_string(), "".to_string()))
                .unwrap();
            return;
        }

        let mut rich_presence_client = DiscordIpcClient::new(
            config
                .discord
                .clone()
                .and_then(|discord| discord.application_id)
                .unwrap_or(String::from("1053747938519679018"))
                .as_str(),
        )
        .expect(
            "Failed to create Discord RPC client, discord is down or the Client ID is invalid.",
        );

        // Start up the client connection, so that we can actually send and receive stuff
        jellyfin_rpc::presence_loop(tx.clone(), Some(rx), &mut rich_presence_client, &config_path, &mut config, VERSION.unwrap_or("0.0.0"), image_urls).await.unwrap_or_else(|_| {
            tx.send(Event::Error("Server Error".to_string(), "Server crashed".to_string())).unwrap()
        });
    });
}
