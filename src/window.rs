use iced::widget::{button, container, text, column, Row, row, Column, checkbox};
use iced::window::resize;
use iced::{executor, Alignment, Length, Size};
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
}

pub struct Gui {
    status: String,
    error: String,
    config: Config,
    panel: Panel,
    blacklist_media_types: BlacklistMediaTypes,
    rx: mpsc::Receiver<String>,
    error_rx: mpsc::Receiver<String>,
    config_rx: mpsc::Receiver<Config>,
    tx: mpsc::Sender<String>,
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
                blacklist_media_types: BlacklistMediaTypes::default(),
                rx: flags.rx.unwrap(),
                error_rx: flags.error_rx.unwrap(),
                config_rx: flags.config_rx.unwrap(),
                tx: flags.tx.unwrap(),
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
                        self.config.jellyfin.blacklist.clone().and_then(|blacklist| {
                            blacklist.media_types.and_then(|media_types| {
                                self.blacklist_media_types.update(media_types);
                                Some(())
                            })
                        });
                    },
                    Err(_) => ()
                }

                match self.error_rx.try_recv() {
                    Ok(error) => self.error = error,
                    Err(_) => ()
                }
            }
            Message::Open(panel) =>  {
                self.panel = panel;
            },
            Message::ToggleMovies(val) => {
                if val {
                    match self.config.jellyfin.blacklist.clone() {
                        Some(mut blacklist) => {
                            match blacklist.media_types {
                                Some(mut media_types) => {
                                    media_types.push(MediaType::Movie)
                                },
                                None => {
                                    blacklist.media_types = Some(vec![MediaType::Movie])
                                },
                            }
                        },
                        None => {
                            self.config.jellyfin.blacklist = Some(Blacklist {
                                media_types: Some(
                                    vec![MediaType::Movie]
                                ),
                                libraries: None,
                            });
                        }
                    }
                    self.blacklist_media_types.movies = true;
                } else {

                }
            },
            Message::ToggleEpisodes(val) => {

            },
            Message::ToggleLiveTv(val) => {

            },
            Message::ToggleMusic(val) => {

            },
            Message::ToggleBooks(val) => {

            },
            Message::ToggleAudioBooks(val) => {

            },
            Message::SaveSettings => self.tx.send("save_settings".to_string()).unwrap()
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

        if self.panel == Panel::Main {
            let start_stop = Row::new()
                .push(button("Start").on_press(Message::Start).padding(10))
                .push(button("Stop").on_press(Message::Stop).padding(10))
                .spacing(10)
                .align_items(Alignment::Center);

            let status = Column::new()
                .push(text("Status: ").size(30))
                .push(text(self.status.clone()))
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

            let mediatypes = Column::new()
                .push(
                    checkbox("Movies", self.blacklist_media_types.movies, Message::ToggleMovies),
                );

            content = column![back, reload_config, mediatypes]
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

#[derive(Default)]
struct BlacklistMediaTypes {
    movies: bool,
    episodes: bool,
    livetv: bool,
    music: bool,
    books: bool,
    audiobooks: bool,
}

impl BlacklistMediaTypes {
    fn update(&mut self, media_types: Vec<MediaType>) {
        for media_type in media_types {
            match media_type {
                MediaType::Episode => self.episodes = true,
                MediaType::LiveTv => self.livetv = true,
                MediaType::Movie => self.movies = true,
                MediaType::Music => self.music = true,
                MediaType::Book => self.books = true,
                MediaType::AudioBook => self.audiobooks = true,
                MediaType::None => (),
            }
        }
        self;
    }
}
