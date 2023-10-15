use iced::{Application, Settings};
use jellyfin_rpc::{
    self,
    core::rpc,
    get_config_path,
    imgur::Imgur,
    jellyfin::{library_check, Content, MediaType},
    Button, Config,
};
use std::sync::mpsc;
mod window;
use clap::Parser;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use retry::retry_with_index;
use window::{Data, Gui};

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(author = "Radical <Radiicall> <radical@radical.fun>")]
#[command(version)]
#[command(about = "Rich presence for Jellyfin", long_about = None)]
struct Args {
    #[arg(short = 'c', long = "config", help = "Path to the config file")]
    config: Option<String>,
    #[arg(
        short = 'i',
        long = "image-urls-file",
        help = "Path to image urls file for imgur"
    )]
    image_urls: Option<String>,
}

#[tokio::main()]
pub async fn main() -> iced::Result {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(|| {
        get_config_path().unwrap_or_else(|err| {
            eprintln!("Error determining config path: {:?}", err);
            std::process::exit(1)
        })
    });

    std::fs::create_dir_all(
        std::path::Path::new(&config_path)
            .parent()
            .expect("Invalid config file path"),
    )
    .ok();

    let mut config = Config::load_config(config_path.clone()).unwrap_or_else(|e| {
        eprintln!(
            "{} {}",
            format_args!(
                "Config can't be loaded: {:?}.\nConfig file should be located at:",
                e
            ),
            config_path
        );
        std::process::exit(2)
    });

    let (tx, rx_iced) = mpsc::channel();
    let (tx_iced, rx) = mpsc::channel();
    let (error_tx, error_rx_iced) = mpsc::channel();

    tokio::spawn(async move {
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
        tx.send("Connecting…".to_string()).unwrap();
        jellyfin_rpc::connect(&mut rich_presence_client);
        tx.send("Listening…".to_string()).unwrap();
        loop {
            while !enabled {
                if rx.try_recv().unwrap_or_else(|_| "".to_string()) == "start" {
                    enabled = true;
                    tx.send("Listening…".to_string()).unwrap();
                }
            }
            let sig = rx.try_recv().unwrap_or_else(|_| "".to_string());

            if sig == "stop" {
                enabled = false;
                tx.send("Stopped".to_string()).unwrap();
            } else if sig == "reload_config" {
                match Config::load_config(config_path.clone()) {
                    Ok(new_config) => {
                        config = new_config;
                        tx.send("Config reloaded!".to_string()).unwrap();
                        error_tx.send("None".to_string()).unwrap();
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                    Err(e) => error_tx
                        .send(format!("Error reloading config: {:?}", e))
                        .unwrap(),
                }
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
                tx.send(format!("{}\n{}", content.details, content.state_message))
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
                        error_tx
                            .send(format!("Failed to use Imgur: {:?}", e))
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
                        error_tx
                            .send(format!("Failed to set activity\nError: {}", err))
                            .unwrap();
                        retry_with_index(retry::delay::Exponential::from_millis(1000), |_| {
                            match rich_presence_client.reconnect() {
                                Ok(result) => retry::OperationResult::Ok(result),
                                Err(_) => retry::OperationResult::Retry(()),
                            }
                        })
                        .unwrap();
                        tx.send(format!("{}\n{}", content.details, content.state_message))
                            .unwrap();
                    });
            } else if connected {
                // Disconnect from the client
                rich_presence_client
                    .clear_activity()
                    .expect("Failed to clear activity");
                // Set connected to false so that we dont try to disconnect again
                connected = false;
                tx.send("Listening…".to_string()).unwrap();
            }

            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    });

    Gui::run(Settings {
        window: iced::window::Settings {
            size: (250, 375),
            resizable: false,
            ..Default::default()
        },
        flags: Data {
            rx: Some(rx_iced),
            error_rx: Some(error_rx_iced),
            tx: Some(tx_iced),
        },
        ..Default::default()
    })
}
