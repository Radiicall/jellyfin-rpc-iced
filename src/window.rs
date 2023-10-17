use crate::server::{self, Event};
use crate::Args;
use clap::Parser;
use iced::widget::{button, checkbox, column, container, row, text, text_input, Column};
use iced::{executor, Alignment, Length};
use iced::{Application, Command, Element, Theme};
use jellyfin_rpc::core::config::{get_config_path, Blacklist, Config};
use jellyfin_rpc::jellyfin::MediaType;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Message {
    Open(Panel),
    ReloadConfig,
    Start,
    Stop,
    Update,
    UpdateUrl(String),
    UpdateApiKey(String),
    ToggleMovies(bool),
    ToggleEpisodes(bool),
    ToggleLiveTv(bool),
    ToggleMusic(bool),
    ToggleBooks(bool),
    ToggleAudioBooks(bool),
    SaveSettings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Main,
    Settings(Setting),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    Main,
    Whitelist,
}

pub struct Gui {
    status: String,
    error: String,
    config: Config,
    panel: Panel,
    whitelist_media_types: WhitelistMediaTypes,
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    config_path: String,
}

impl Application for Gui {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Gui, Command<Message>) {
        let args = Args::parse();

        let config_path = match args.config.clone() {
            Some(path) => path,
            None => get_config_path().unwrap_or_else(|err| {
                eprintln!("Error determining config path: {:?}", err);
                std::process::exit(1)
            }),
        };

        std::fs::create_dir_all(
            std::path::Path::new(&config_path)
                .parent()
                .expect("Invalid config file path"),
        )
        .ok();

        let (tx_server, rx_iced) = mpsc::channel();
        let (tx_iced, rx_server) = mpsc::channel();

        let config = Config::load(&config_path).unwrap_or_else(|_| Config::default());

        (
            Gui {
                status: "Not running".to_string(),
                error: "None".to_string(),
                config: config.clone(),
                panel: Panel::Main,
                whitelist_media_types: WhitelistMediaTypes::default(),
                rx: rx_iced,
                tx: tx_iced,
                config_path: config_path.clone(),
            },
            Command::perform(
                server::run(config_path, config, args, tx_server, rx_server),
                |_| Message::Start,
            ),
        )
    }

    fn title(&self) -> String {
        String::from("Jellyfin-RPC-Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ReloadConfig => {
                match Config::load(&self.config_path) {
                    Ok(config) => {
                        self.config = config;
                    }
                    Err(_) => self.config = Config::default(),
                }
                self.whitelist_media_types.update(&self.config);
                let _ = self.tx.send(Event::ReloadConfig);
                Command::none()
            }
            Message::Start => match self.tx.send(Event::Start) {
                Ok(()) => Command::none(),
                Err(_) => Command::none(),
            },
            Message::Stop => match self.tx.send(Event::Stop) {
                Ok(()) => Command::none(),
                Err(_) => Command::none(),
            },
            Message::Update => {
                match self.rx.try_recv() {
                    Ok(Event::Status(status)) => self.status = status,
                    Ok(Event::Error(error)) => self.error = error,
                    Ok(_) => (),
                    Err(_) => (),
                }
                Command::none()
            }
            Message::Open(panel) => {
                if panel == Panel::Main {
                    match Config::load(&self.config_path) {
                        Ok(config) => {
                            self.config = config;
                        }
                        Err(_) => self.config = Config::default(),
                    };

                    self.whitelist_media_types.update(&self.config);
                }

                self.panel = panel;
                Command::none()
            }
            Message::UpdateUrl(url) => {
                self.config.jellyfin.url = url;
                Command::none()
            }
            Message::UpdateApiKey(api_key) => {
                self.config.jellyfin.api_key = api_key;
                Command::none()
            }
            Message::ToggleMovies(val) => {
                media_type_toggle(val, self, MediaType::Movie);
                Command::none()
            }
            Message::ToggleEpisodes(val) => {
                media_type_toggle(val, self, MediaType::Episode);
                Command::none()
            }
            Message::ToggleLiveTv(val) => {
                media_type_toggle(val, self, MediaType::LiveTv);
                Command::none()
            }
            Message::ToggleMusic(val) => {
                media_type_toggle(val, self, MediaType::Music);
                Command::none()
            }
            Message::ToggleBooks(val) => {
                media_type_toggle(val, self, MediaType::Book);
                Command::none()
            }
            Message::ToggleAudioBooks(val) => {
                media_type_toggle(val, self, MediaType::AudioBook);
                Command::none()
            }
            Message::SaveSettings => {
                match std::fs::write(
                    &self.config_path,
                    serde_json::to_string_pretty(&self.config).unwrap(),
                ) {
                    Ok(()) => {
                        self.tx.send(Event::ReloadConfig).ok();
                    }
                    Err(err) => self.error = format!("{:?}", err),
                }
                Command::none()
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::time::every(std::time::Duration::from_millis(500)).map(|_| Message::Update)
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn view(&self) -> Element<Message> {
        let mut content = Column::new();

        let status = column![text("Status: ").size(30), text(self.status.clone()),]
            .align_items(Alignment::Center);

        if self.panel == Panel::Main {
            let start_stop = row![
                button("Start").on_press(Message::Start).padding(10),
                button("Stop").on_press(Message::Stop).padding(10),
            ]
            .spacing(10)
            .align_items(Alignment::Center);

            let error = column![text("Error: ").size(30), text(self.error.clone()),]
                .spacing(10)
                .align_items(Alignment::Center);

            let settings = button("Settings")
                .on_press(Message::Open(Panel::Settings(Setting::Main)))
                .padding(5);

            content = column![start_stop, status, error, settings]
                .spacing(10)
                .align_items(Alignment::Center);
        } else if self.panel == Panel::Settings(Setting::Main) {
            let back = column![
                button("< Back")
                    .on_press(Message::Open(Panel::Main))
                    .padding(5),
                button("MediaTypes >")
                    .on_press(Message::Open(Panel::Settings(Setting::Whitelist)))
                    .padding(5),
                button("Libraries >").padding(5),
            ]
            .spacing(3)
            .align_items(Alignment::Center);

            let reload_config = button("Reload Config")
                .on_press(Message::ReloadConfig)
                .padding(10);

            let url = row![
                text("URL:"),
                text_input("http://localhost:8096", &self.config.jellyfin.url)
                    .on_input(Message::UpdateUrl),
            ]
            .spacing(3)
            .align_items(Alignment::Center);

            let api_key = row![
                text("Api Key:"),
                text_input("aaaabbbbcccc111122223333", &self.config.jellyfin.api_key)
                    .on_input(Message::UpdateApiKey),
            ]
            .spacing(3)
            .align_items(Alignment::Center);

            let save = button("Save").on_press(Message::SaveSettings).padding(10);

            content = column![back, reload_config, url, api_key, save, status]
                .spacing(10)
                .align_items(Alignment::Center);
        } else if self.panel == Panel::Settings(Setting::Whitelist) {
            let back = row![button("< Back")
                .on_press(Message::Open(Panel::Settings(Setting::Main)))
                .padding(5),]
            .spacing(3)
            .align_items(Alignment::Center);

            let mediatypes = column![
                checkbox(
                    "Movies",
                    self.whitelist_media_types.movies,
                    Message::ToggleMovies
                ),
                checkbox(
                    "Episodes",
                    self.whitelist_media_types.episodes,
                    Message::ToggleEpisodes
                ),
                checkbox(
                    "Television",
                    self.whitelist_media_types.livetv,
                    Message::ToggleLiveTv
                ),
                checkbox(
                    "Music",
                    self.whitelist_media_types.music,
                    Message::ToggleMusic
                ),
                checkbox(
                    "Books",
                    self.whitelist_media_types.books,
                    Message::ToggleBooks
                ),
                checkbox(
                    "AudioBooks",
                    self.whitelist_media_types.audiobooks,
                    Message::ToggleAudioBooks
                ),
            ]
            .spacing(6)
            .align_items(Alignment::Start);

            content = column![back, mediatypes]
                .spacing(10)
                .align_items(Alignment::Center);
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(20)
            .into()
    }
}

struct WhitelistMediaTypes {
    movies: bool,
    episodes: bool,
    livetv: bool,
    music: bool,
    books: bool,
    audiobooks: bool,
}

impl Default for WhitelistMediaTypes {
    fn default() -> Self {
        Self {
            movies: true,
            episodes: true,
            livetv: true,
            music: true,
            books: true,
            audiobooks: true,
        }
    }
}

impl WhitelistMediaTypes {
    fn update(&mut self, config: &Config) {
        self.movies = true;
        self.episodes = true;
        self.livetv = true;
        self.music = true;
        self.books = true;
        self.audiobooks = true;

        match &config.jellyfin.blacklist {
            Some(blacklist) => match &blacklist.media_types {
                Some(media_types) => {
                    for media_type in media_types {
                        match media_type {
                            MediaType::Episode => self.episodes = false,
                            MediaType::LiveTv => self.livetv = false,
                            MediaType::Movie => self.movies = false,
                            MediaType::Music => self.music = false,
                            MediaType::Book => self.books = false,
                            MediaType::AudioBook => self.audiobooks = false,
                            MediaType::None => (),
                        }
                    }
                }
                None => (),
            },
            None => (),
        }
    }
}

fn media_type_toggle(val: bool, gui: &mut Gui, media_type: MediaType) {
    match media_type {
        MediaType::Episode => gui.whitelist_media_types.episodes = val,
        MediaType::LiveTv => gui.whitelist_media_types.livetv = val,
        MediaType::Movie => gui.whitelist_media_types.movies = val,
        MediaType::Music => gui.whitelist_media_types.music = val,
        MediaType::Book => gui.whitelist_media_types.books = val,
        MediaType::AudioBook => gui.whitelist_media_types.audiobooks = val,
        MediaType::None => (),
    }

    if val {
        gui.config
            .jellyfin
            .blacklist
            .as_mut()
            .unwrap()
            .media_types
            .as_mut()
            .unwrap()
            .retain(|mt| mt != &media_type);
    } else {
        match gui.config.jellyfin.blacklist.clone() {
            Some(blacklist) => match blacklist.media_types {
                Some(mut media_types) => {
                    media_types.push(media_type);
                    gui.config.jellyfin.blacklist.as_mut().unwrap().media_types = Some(media_types);
                }
                None => {
                    gui.config.jellyfin.blacklist.as_mut().unwrap().media_types =
                        Some(vec![media_type])
                }
            },
            None => {
                gui.config.jellyfin.blacklist = Some(Blacklist {
                    media_types: Some(vec![media_type]),
                    libraries: None,
                });
            }
        }
    }
}
