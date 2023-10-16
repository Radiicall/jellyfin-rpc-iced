use iced::widget::{button, container, text, column, Row, Column, checkbox, text_input};
use iced::{executor, Alignment, Length};
use iced::{Application, Command, Element, Theme};
use jellyfin_rpc::core::config::{Config, Jellyfin, Username, Blacklist};
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
    Settings
}

#[derive(Default)]
pub struct Data {
    pub rx: Option<mpsc::Receiver<String>>,
    pub error_rx: Option<mpsc::Receiver<String>>,
    pub config_rx: Option<mpsc::Receiver<Config>>,
    pub tx: Option<mpsc::Sender<String>>,
    pub config_tx: Option<mpsc::Sender<Config>>,
}

pub struct Gui {
    status: String,
    error: String,
    config: Config,
    panel: Panel,
    whitelist_media_types: WhitelistMediaTypes,
    rx: mpsc::Receiver<String>,
    error_rx: mpsc::Receiver<String>,
    config_rx: mpsc::Receiver<Config>,
    tx: mpsc::Sender<String>,
    config_tx: mpsc::Sender<Config>,
}

impl Application for Gui {
    type Executor = executor::Default;
    type Flags = Data;
    type Message = Message;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Gui, Command<Message>) {
        (
            Gui {
                status: "Unknown".to_string(),
                error: "None".to_string(),
                config: Config {
                    jellyfin: Jellyfin {
                        url: "none".to_string(),
                        username: Username::String("none".to_string()),
                        api_key: "none".to_string(),
                        music: None,
                        blacklist: None,
                    },
                    discord: None,
                    imgur: None,
                    images: None,
                },
                panel: Panel::Main,
                whitelist_media_types: WhitelistMediaTypes::default(),
                rx: flags.rx.unwrap(),
                error_rx: flags.error_rx.unwrap(),
                config_rx: flags.config_rx.unwrap(),
                tx: flags.tx.unwrap(),
                config_tx: flags.config_tx.unwrap(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Jellyfin-RPC-Iced")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ReloadConfig => self.tx.send("reload_config".to_string()).unwrap(),
            Message::Start => self.tx.send("start".to_string()).unwrap(),
            Message::Stop => self.tx.send("stop".to_string()).unwrap(),
            Message::Update => {
                match self.rx.try_recv() {
                    Ok(status) => self.status = status,
                    Err(_) => ()
                }

                match self.config_rx.try_recv() {
                    Ok(config) => {
                        self.config = config;
                        self.whitelist_media_types.update(&self.config);
                    },
                    Err(_) => ()
                }

                match self.error_rx.try_recv() {
                    Ok(error) => self.error = error,
                    Err(_) => ()
                }
            }
            Message::Open(panel) =>  {
                match panel {
                    Panel::Main => {
                        self.tx.send("reload_config".to_string()).unwrap()
                    },
                    _ => ()
                }

                self.panel = panel;
            },
            Message::UpdateUrl(url) => {
                self.config.jellyfin.url = url;
            },
            Message::UpdateApiKey(api_key) => {
                self.config.jellyfin.api_key = api_key;
            },
            Message::ToggleMovies(val) => {
                media_type_toggle(val, self, MediaType::Movie)
            },
            Message::ToggleEpisodes(val) => {
                media_type_toggle(val, self, MediaType::Episode)
            },
            Message::ToggleLiveTv(val) => {
                media_type_toggle(val, self, MediaType::LiveTv)
            },
            Message::ToggleMusic(val) => {
                media_type_toggle(val, self, MediaType::Music)
            },
            Message::ToggleBooks(val) => {
                media_type_toggle(val, self, MediaType::Book)
            },
            Message::ToggleAudioBooks(val) => {
                media_type_toggle(val, self, MediaType::AudioBook)
            },
            Message::SaveSettings => {
                self.config_tx.send(self.config.clone()).unwrap();
                self.tx.send("save_config".to_string()).unwrap()
            }
        };
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Update)
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn view(&self) -> Element<Message> {
        let mut content = Column::new();

        let status = Column::new()
            .push(text("Status: ").size(30))
            .push(text(self.status.clone()))
            .align_items(Alignment::Center);

        if self.panel == Panel::Main {
            let start_stop = Row::new()
                .push(button("Start").on_press(Message::Start).padding(10))
                .push(button("Stop").on_press(Message::Stop).padding(10))
                .spacing(10)
                .align_items(Alignment::Center);

            let error = Column::new()
                .push(text("Error: ").size(30))
                .push(text(self.error.clone()))
                .spacing(10)
                .align_items(Alignment::Center);

            let settings = button("Settings")
                .on_press(Message::Open(Panel::Settings))
                .padding(5);

            content = column![start_stop, status, error, settings]
                .spacing(10)
                .align_items(Alignment::Center);
        } else if self.panel == Panel::Settings {
            let back = button("< Back")
                .on_press(Message::Open(Panel::Main))
                .padding(5);

            let reload_config = button("Reload Config")
                .on_press(Message::ReloadConfig)
                .padding(10);

            let url = Row::new()
                .push(
                    text("URL:")
                )
                .push(
                    text_input("http://localhost:8096", &self.config.jellyfin.url)
                        .on_input(Message::UpdateUrl)
                )
                .spacing(3)
                .align_items(Alignment::Center);

            let api_key = Row::new()
                .push(
                    text("Api Key:")
                )
                .push(
                    text_input("aaaabbbbcccc111122223333", &self.config.jellyfin.api_key)
                        .on_input(Message::UpdateApiKey)
                )
                .spacing(3)
                .align_items(Alignment::Center);

            let mediatypes = Column::new()
                .push(
                    checkbox("Movies", self.whitelist_media_types.movies, Message::ToggleMovies),
                )
                .push(
                    checkbox("Episodes", self.whitelist_media_types.episodes, Message::ToggleEpisodes),
                )
                .push(
                    checkbox("Television", self.whitelist_media_types.livetv, Message::ToggleLiveTv),
                )
                .push(
                    checkbox("Music", self.whitelist_media_types.music, Message::ToggleMusic),
                )
                .push(
                    checkbox("Books", self.whitelist_media_types.books, Message::ToggleBooks),
                )
                .push(
                    checkbox("AudioBooks", self.whitelist_media_types.audiobooks, Message::ToggleAudioBooks),
                )
                .spacing(6)
                .align_items(Alignment::Start);

            let save = button("Save")
                .on_press(Message::SaveSettings)
                .padding(10);

            content = column![back, reload_config, url, api_key, mediatypes, save, status]
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
            Some(blacklist) => {
                match &blacklist.media_types {
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
                    },
                    None => ()
                }
            }
            None => ()
        }
    }
}

fn media_type_toggle(val:bool, gui: &mut Gui, media_type: MediaType) {
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
        gui.config.jellyfin.blacklist.as_mut().unwrap().media_types.as_mut().unwrap().retain(|mt| mt != &media_type);
    } else {
        match gui.config.jellyfin.blacklist.clone() {
            Some(blacklist) => {
                match blacklist.media_types {
                    Some(mut media_types) => {
                        media_types.push(media_type);
                        gui.config.jellyfin.blacklist.as_mut().unwrap().media_types = Some(media_types);
                    },
                    None => {
                        gui.config.jellyfin.blacklist.as_mut().unwrap().media_types = Some(vec![media_type])
                    },
                }
            },
            None => {
                gui.config.jellyfin.blacklist = Some(Blacklist {
                    media_types: Some(
                        vec![media_type]
                    ),
                    libraries: None,
                });
            }
        }
        
    }
}
