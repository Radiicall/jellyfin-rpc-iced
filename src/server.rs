use crate::{Args, VERSION};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use jellyfin_rpc::{
    self,
    core::{config::Username, rpc},
    jellyfin::{library_check, Content, MediaType},
    services::imgur::Imgur,
    Button, Config,
};
use retry::retry_with_index;
use std::sync::mpsc;

pub async fn run(
    config_path: String,
    mut config: Config,
    args: Args,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
) {
    tokio::spawn(async move {
        if config.jellyfin.api_key.is_empty() {
            tx.send(Event::Error("Jellyfin API key not set".to_string()))
                .unwrap();
            return;
        } else if config.jellyfin.url.is_empty() {
            tx.send(Event::Error("Jellyfin URL not set".to_string()))
                .unwrap();
            return;
        } else if config.jellyfin.username == Username::String("".to_string()) {
            tx.send(Event::Error("Jellyfin Username not set".to_string()))
                .unwrap();
            return;
        }

        let mut enabled: bool = true;
        let mut connected: bool = false;
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
        tx.send(Event::Status("Connecting…".to_string())).unwrap();
        jellyfin_rpc::connect(&mut rich_presence_client);
        tx.send(Event::Status("Listening…".to_string())).unwrap();
        loop {
            while !enabled {
                if rx.try_recv().unwrap_or(Event::Unknown) == Event::Start {
                    enabled = true;
                    tx.send(Event::Status("Listening…".to_string())).unwrap();
                }
            }

            // Handle the signal
            match rx.try_recv() {
                Ok(Event::Stop) => {
                    enabled = false;
                    tx.send(Event::Status("Stopped".to_string())).unwrap();
                }
                Ok(Event::Start) => (),
                Ok(Event::ReloadConfig) => match Config::load(&config_path) {
                    Ok(new_config) => {
                        config = new_config;

                        tx.send(Event::Status("Config reloaded!".to_string()))
                            .unwrap();
                        tx.send(Event::Error("None".to_string())).unwrap();

                        std::thread::sleep(std::time::Duration::from_secs(2));

                        tx.send(Event::Status("Listening…".to_string())).unwrap();
                    }
                    Err(e) => tx
                        .send(Event::Error(format!("Error reloading config: {:?}", e)))
                        .unwrap(),
                },
                /*
                Ok(val) if val == "save_config" => {
                    config = config_rx.recv().unwrap();
                    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

                    tx.send("Settings saved!".to_string()).unwrap();

                    std::thread::sleep(std::time::Duration::from_secs(2));

                    tx.send("Listening…".to_string()).unwrap();
                },
                */
                Ok(_) => (),
                Err(_) => (),
            }

            let mut content = Content::get(&config).await.unwrap();

            let mut blacklist_check = true;
            config
                .clone()
                .jellyfin
                .blacklist
                .and_then(|blacklist| blacklist.media_types)
                .unwrap_or(vec![MediaType::None])
                .iter()
                .for_each(|x| {
                    if blacklist_check && !content.media_type.is_none() {
                        blacklist_check = content.media_type != *x
                    }
                });
            if config
                .clone()
                .jellyfin
                .blacklist
                .and_then(|blacklist| blacklist.libraries)
                .is_some()
            {
                for library in &config
                    .clone()
                    .jellyfin
                    .blacklist
                    .and_then(|blacklist| blacklist.libraries)
                    .unwrap()
                {
                    if blacklist_check && !content.media_type.is_none() {
                        blacklist_check = library_check(
                            &config.jellyfin.url,
                            &config.jellyfin.api_key,
                            &content.item_id,
                            library,
                        )
                        .await
                        .unwrap();
                    }
                }
            }

            if !content.media_type.is_none() && blacklist_check && enabled {
                // Print what we're watching
                if !connected {
                    // Set connected to true so that we don't try to connect again
                    connected = true;
                }
                tx.send(Event::Status(format!(
                    "{}\n{}",
                    content.details, content.state_message
                )))
                .unwrap();
                if config
                    .clone()
                    .images
                    .and_then(|images| images.imgur_images)
                    .unwrap_or(false)
                    && content.media_type != MediaType::LiveTv
                {
                    content.image_url = Imgur::get(
                        &content.image_url,
                        &content.item_id,
                        &config
                            .clone()
                            .imgur
                            .and_then(|imgur| imgur.client_id)
                            .expect("Imgur client ID cant be loaded."),
                        args.image_urls.clone(),
                    )
                    .await
                    .unwrap_or_else(|e| {
                        tx.send(Event::Error(format!("Failed to use Imgur: {:?}", e)))
                            .unwrap();
                        Imgur::default()
                    })
                    .url;
                }

                // Set the activity
                let mut rpcbuttons: Vec<activity::Button> = vec![];
                let mut x = 0;
                let default_button = Button {
                    name: String::from("dynamic"),
                    url: String::from("dynamic"),
                };
                let buttons = config
                    .clone()
                    .discord
                    .and_then(|discord| discord.buttons)
                    .unwrap_or(vec![default_button.clone(), default_button]);

                // For loop to determine if external services are to be used or if there are custom buttons instead
                for button in buttons.iter() {
                    if button.name == "dynamic"
                        && button.url == "dynamic"
                        && content.external_services.len() != x
                    {
                        rpcbuttons.push(activity::Button::new(
                            &content.external_services[x].name,
                            &content.external_services[x].url,
                        ));
                        x += 1
                    } else if button.name != "dynamic" || button.url != "dynamic" {
                        rpcbuttons.push(activity::Button::new(&button.name, &button.url))
                    }
                }

                rich_presence_client
                    .set_activity(rpc::setactivity(
                        &content.state_message,
                        &content.details,
                        content.endtime,
                        &content.image_url,
                        rpcbuttons,
                        format!("Jellyfin-RPC-Iced v{}", VERSION.unwrap_or("0.0.0")).as_str(),
                        &content.media_type,
                    ))
                    .unwrap_or_else(|err| {
                        tx.send(Event::Error(format!(
                            "Failed to set activity\nError: {}",
                            err
                        )))
                        .unwrap();
                        retry_with_index(retry::delay::Exponential::from_millis(1000), |_| {
                            match rich_presence_client.reconnect() {
                                Ok(result) => retry::OperationResult::Ok(result),
                                Err(_) => retry::OperationResult::Retry(()),
                            }
                        })
                        .unwrap();
                        tx.send(Event::Status(format!(
                            "{}\n{}",
                            content.details, content.state_message
                        )))
                        .unwrap();
                    });
            } else if connected {
                // Disconnect from the client
                rich_presence_client
                    .clear_activity()
                    .expect("Failed to clear activity");
                // Set connected to false so that we dont try to disconnect again
                connected = false;
                tx.send(Event::Status("Listening…".to_string())).unwrap();
            }

            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });
}

#[derive(PartialEq)]
pub enum Event {
    ReloadConfig,
    Stop,
    Start,
    Error(String),
    Status(String),
    Unknown,
}
