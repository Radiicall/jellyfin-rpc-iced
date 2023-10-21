use crate::server::{self, Event};
use crate::Args;
use clap::Parser;
use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{executor, Alignment, Length};
use iced::{Application, Command, Element, Theme};
use jellyfin_rpc::core::config::{
    get_config_path, Blacklist, Config, Discord, Images, Imgur, Username,
};
use jellyfin_rpc::jellyfin::MediaType;
use jellyfin_rpc::Button;
use serde_json::Value;
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
    ToggleMediaType(MediaType, bool),
    ToggleCustomButtons(bool),
    UpdateButtonOneName(String),
    UpdateButtonOneUrl(String),
    UpdateButtonTwoName(String),
    UpdateButtonTwoUrl(String),
    UpdateNewUsername(String),
    AddUsername,
    RemoveUsername(String),
    Images(bool),
    Imgur(bool),
    ImgurClientId(String),
    UpdateLibraries(Vec<String>),
    ToggleLibrary(Library, bool),
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
    MediaTypes,
    Buttons,
    Users,
    Images,
    Libraries,
}

pub struct Gui {
    status: String,
    error: String,
    config: Config,
    panel: Panel,
    whitelist_media_types: WhitelistMediaTypes,
    custom_buttons: bool,
    buttons: Buttons,
    image_options: ImageOptions,
    new_username: String,
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    libraries: Vec<Library>,
    config_path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Library {
    name: String,
    enabled: bool,
}

impl Gui {
    fn media_type_toggle(&mut self, val: bool, media_type: MediaType) {
        match media_type {
            MediaType::Episode => self.whitelist_media_types.episodes = val,
            MediaType::LiveTv => self.whitelist_media_types.livetv = val,
            MediaType::Movie => self.whitelist_media_types.movies = val,
            MediaType::Music => self.whitelist_media_types.music = val,
            MediaType::Book => self.whitelist_media_types.books = val,
            MediaType::AudioBook => self.whitelist_media_types.audiobooks = val,
            MediaType::None => (),
        }
    
        if val {
            self.config
                .jellyfin
                .blacklist
                .as_mut()
                .unwrap()
                .media_types
                .as_mut()
                .unwrap()
                .retain(|mt| mt != &media_type);
        } else {
            match self.config.jellyfin.blacklist.clone() {
                Some(blacklist) => match blacklist.media_types {
                    Some(mut media_types) => {
                        media_types.push(media_type);
                        self.config.jellyfin.blacklist.as_mut().unwrap().media_types = Some(media_types);
                    }
                    None => {
                        self.config.jellyfin.blacklist.as_mut().unwrap().media_types =
                            Some(vec![media_type])
                    }
                },
                None => {
                    self.config.jellyfin.blacklist = Some(Blacklist {
                        media_types: Some(vec![media_type]),
                        libraries: None,
                    });
                }
            }
        }
    }
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

        let custom_buttons = false;

        let default_button = Button {
            name: String::from("dynamic"),
            url: String::from("dynamic"),
        };

        let buttons = config
            .discord
            .clone()
            .and_then(|discord| discord.buttons)
            .unwrap_or(vec![default_button.clone(), default_button]);

        (
            Gui {
                status: "Not running".to_string(),
                error: "None".to_string(),
                config: config.clone(),
                panel: Panel::Main,
                whitelist_media_types: WhitelistMediaTypes::default(),
                new_username: "".to_string(),
                rx: rx_iced,
                tx: tx_iced,
                custom_buttons,
                buttons: Buttons {
                    one: buttons[0].clone(),
                    two: buttons[1].clone(),
                },
                image_options: ImageOptions {
                    enabled: false,
                    imgur: false,
                    imgur_client_id: "".to_string(),
                },
                libraries: Vec::new(),
                config_path: config_path.clone(),
            },
            Command::perform(
                server::run(config_path, config, args, tx_server, rx_server),
                |_| Message::Open(Panel::Main),
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
                self.panel = panel;

                if self.panel == Panel::Main {
                    match Config::load(&self.config_path) {
                        Ok(config) => {
                            self.config = config;
                        }
                        Err(_) => self.config = Config::default(),
                    };

                    self.whitelist_media_types.update(&self.config);

                    self.custom_buttons = self.buttons.one.name != "dynamic"
                        || self.buttons.one.url != "dynamic"
                        || self.buttons.two.name != "dynamic"
                        || self.buttons.two.url != "dynamic";

                    let default_button = Button {
                        name: String::from("dynamic"),
                        url: String::from("dynamic"),
                    };

                    let buttons = self
                        .config
                        .discord
                        .clone()
                        .and_then(|discord| discord.buttons)
                        .unwrap_or(vec![default_button.clone(), default_button]);

                    self.buttons.update(buttons);

                    self.image_options.enabled =
                        self.config.images.clone().is_some_and(|images| {
                            images.enable_images.is_some_and(|enabled| enabled)
                        });

                    if self
                        .config
                        .images
                        .clone()
                        .is_some_and(|images| images.imgur_images.is_some_and(|imgur| imgur))
                    {
                        self.image_options.imgur = true;

                        self.image_options.imgur_client_id = self
                            .config
                            .imgur
                            .clone()
                            .and_then(|imgur| imgur.client_id)
                            .unwrap_or_default();
                    } else {
                        self.image_options.imgur = false;
                    }

                    return Command::perform(
                        get_libraries(
                            self.config.jellyfin.url.clone(),
                            self.config.jellyfin.api_key.clone(),
                        ),
                        |libraries| Message::UpdateLibraries(libraries.unwrap()),
                    );
                }

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
            Message::ToggleMediaType(media_type, val) => {
                self.media_type_toggle(val, media_type);
                Command::none()
            }
            Message::ToggleCustomButtons(val) => {
                self.custom_buttons = val;
                Command::none()
            }
            Message::UpdateButtonOneName(name) => {
                self.buttons.one.name = name;
                Command::none()
            }
            Message::UpdateButtonOneUrl(url) => {
                self.buttons.one.url = url;
                Command::none()
            }
            Message::UpdateButtonTwoName(name) => {
                self.buttons.two.name = name;
                Command::none()
            }
            Message::UpdateButtonTwoUrl(url) => {
                self.buttons.two.url = url;
                Command::none()
            }
            Message::UpdateNewUsername(username) => {
                self.new_username = username;
                Command::none()
            }
            Message::AddUsername => {
                let mut usernames = match &self.config.jellyfin.username {
                    Username::Vec(usernames) => usernames.to_vec(),
                    Username::String(username) => vec![username.to_string()],
                };

                if !usernames.contains(&self.new_username) {
                    usernames.push(self.new_username.clone());
                }

                self.new_username = "".to_string();
                self.config.jellyfin.username = Username::Vec(usernames);

                Command::none()
            }
            Message::RemoveUsername(pattern) => {
                let mut usernames = match &self.config.jellyfin.username {
                    Username::Vec(usernames) => usernames.to_vec(),
                    Username::String(username) => vec![username.to_string()],
                };

                usernames.retain(|username| username != &pattern);

                self.config.jellyfin.username = Username::Vec(usernames);

                Command::none()
            }
            Message::Images(val) => {
                self.image_options.enabled = val;
                Command::none()
            }
            Message::Imgur(val) => {
                self.image_options.imgur = val;
                Command::none()
            }
            Message::ImgurClientId(client_id) => {
                self.image_options.imgur_client_id = client_id;
                Command::none()
            }
            Message::UpdateLibraries(libraries) => {
                self.libraries = libraries
                    .iter()
                    .map(|library| Library {
                        name: library.to_string(),
                        enabled: true,
                    })
                    .collect();

                if let Some(blacklist) = self.config.jellyfin.blacklist.clone() {
                    if let Some(libraries) = blacklist.libraries {
                        for library in &mut self.libraries {
                            if libraries.contains(&library.name) {
                                library.enabled = false;
                            }
                        }
                    }
                }

                Command::none()
            }
            Message::ToggleLibrary(library, val) => {
                for _library in &mut self.libraries {
                    if library == *_library {
                        _library.enabled = val;
                    }
                }
                Command::none()
            }
            Message::SaveSettings => {
                if self.config.discord.is_some() {
                    let mut discord = self.config.discord.clone().unwrap();
                    discord.buttons =
                        Some(vec![self.buttons.one.clone(), self.buttons.two.clone()]);
                    self.config.discord = Some(discord);
                } else {
                    self.config.discord = Some(Discord {
                        application_id: None,
                        buttons: Some(vec![self.buttons.one.clone(), self.buttons.two.clone()]),
                    })
                }

                self.config.images = Some(Images {
                    enable_images: Some(self.image_options.enabled),
                    imgur_images: Some(self.image_options.imgur),
                });

                self.config.imgur = Some(Imgur {
                    client_id: Some(self.image_options.imgur_client_id.clone()),
                });

                match self.config.jellyfin.blacklist.clone() {
                    Some(_) => {
                        self.config.jellyfin.blacklist = Some(Blacklist {
                            media_types: self
                                .config
                                .jellyfin
                                .blacklist
                                .clone()
                                .unwrap()
                                .media_types,
                            libraries: Some(
                                self.libraries
                                    .iter()
                                    .filter(|library| !library.enabled)
                                    .map(|library| library.name.to_owned())
                                    .collect(),
                            ),
                        })
                    }
                    None => {
                        self.config.jellyfin.blacklist = Some(Blacklist {
                            media_types: None,
                            libraries: Some(
                                self.libraries
                                    .iter()
                                    .filter(|library| !library.enabled)
                                    .map(|library| library.name.to_owned())
                                    .collect(),
                            ),
                        })
                    }
                };

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
        let status = column![text("Status: ").size(30), text(self.status.clone()),]
            .align_items(Alignment::Center);

        let content = match &self.panel {
            Panel::Main => {
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

                column![start_stop, status, error, settings]
                    .spacing(10)
                    .align_items(Alignment::Center)
            }
            Panel::Settings(setting) => match setting {
                Setting::Main => {
                    let menu_buttons = column![
                        row![
                            button("< Back")
                                .on_press(Message::Open(Panel::Main))
                                .padding(5),
                            button("Users >")
                                .on_press(Message::Open(Panel::Settings(Setting::Users)))
                                .padding(5),
                        ]
                        .spacing(3)
                        .align_items(Alignment::Start),
                        row![
                            button("Buttons >")
                                .on_press(Message::Open(Panel::Settings(Setting::Buttons)))
                                .padding(5),
                            button("Images >")
                                .on_press(Message::Open(Panel::Settings(Setting::Images)))
                                .padding(5),
                        ]
                        .spacing(3)
                        .align_items(Alignment::Start),
                        row![
                            button("MediaTypes >")
                                .on_press(Message::Open(Panel::Settings(Setting::MediaTypes)))
                                .padding(5),
                            button("Libraries >")
                                .on_press(Message::Open(Panel::Settings(Setting::Libraries)))
                                .padding(5),
                        ]
                        .spacing(3)
                        .align_items(Alignment::Start)
                    ]
                    .spacing(3)
                    .align_items(Alignment::Start);

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

                    column![menu_buttons, reload_config, url, api_key, save, status]
                        .spacing(10)
                        .align_items(Alignment::Center)
                }
                Setting::MediaTypes => {
                    let back: iced::widget::Row<'_, Message> = row![button("< Back")
                        .on_press(Message::Open(Panel::Settings(Setting::Main)))
                        .padding(5),]
                    .spacing(3)
                    .align_items(Alignment::Center);

                    let mediatypes = column![
                        checkbox(
                            "Movies",
                            self.whitelist_media_types.movies,
                            |val| Message::ToggleMediaType(MediaType::Movie, val)
                        ),
                        checkbox(
                            "Episodes",
                            self.whitelist_media_types.episodes,
                            |val| Message::ToggleMediaType(MediaType::Episode, val)
                        ),
                        checkbox(
                            "Television",
                            self.whitelist_media_types.livetv,
                            |val| Message::ToggleMediaType(MediaType::LiveTv, val)
                        ),
                        checkbox(
                            "Music",
                            self.whitelist_media_types.music,
                            |val| Message::ToggleMediaType(MediaType::Music, val)
                        ),
                        checkbox(
                            "Books",
                            self.whitelist_media_types.books,
                            |val| Message::ToggleMediaType(MediaType::Book, val)
                        ),
                        checkbox(
                            "AudioBooks",
                            self.whitelist_media_types.audiobooks,
                            |val| Message::ToggleMediaType(MediaType::AudioBook, val)
                        ),
                    ]
                    .spacing(6)
                    .align_items(Alignment::Start);

                    column![back, mediatypes]
                        .spacing(10)
                        .align_items(Alignment::Center)
                }
                Setting::Buttons => {
                    let back = row![button("< Back")
                        .on_press(Message::Open(Panel::Settings(Setting::Main)))
                        .padding(5),]
                    .spacing(3)
                    .align_items(Alignment::Center);

                    let custom = checkbox(
                        "Custom buttons",
                        self.custom_buttons,
                        Message::ToggleCustomButtons,
                    );

                    let buttons = self
                        .custom_buttons
                        .then_some(
                            column![
                                text("Button 1").size(20),
                                column![
                                    row![
                                        text("Name: "),
                                        text_input("My cool website", &self.buttons.one.name)
                                            .on_input(Message::UpdateButtonOneName)
                                    ]
                                    .align_items(Alignment::Center),
                                    row![
                                        text("URL: "),
                                        text_input("https://example.com", &self.buttons.one.url)
                                            .on_input(Message::UpdateButtonOneUrl)
                                    ]
                                    .align_items(Alignment::Center)
                                ]
                                .align_items(Alignment::Center),
                                text("Button 2").size(20),
                                column![
                                    row![
                                        text("Name: "),
                                        text_input("My 2nd cool website", &self.buttons.two.name)
                                            .on_input(Message::UpdateButtonTwoName)
                                    ]
                                    .align_items(Alignment::Center),
                                    row![
                                        text("URL: "),
                                        text_input("https://example.org", &self.buttons.two.url)
                                            .on_input(Message::UpdateButtonTwoUrl)
                                    ]
                                    .align_items(Alignment::Center)
                                ]
                                .align_items(Alignment::Center)
                            ]
                            .align_items(Alignment::Center),
                        )
                        .unwrap_or_default();

                    column![back, custom, buttons]
                        .spacing(10)
                        .align_items(Alignment::Center)
                }
                Setting::Users => {
                    let back = row![button("< Back")
                        .on_press(Message::Open(Panel::Settings(Setting::Main)))
                        .padding(5),]
                    .spacing(3)
                    .align_items(Alignment::Center);

                    let add = row![
                        text("New: "),
                        text_input("Press enter to submit", &self.new_username)
                            .on_input(Message::UpdateNewUsername)
                            .on_submit(Message::AddUsername)
                    ]
                    .align_items(Alignment::Center);

                    let usernames = match &self.config.jellyfin.username {
                        Username::Vec(usernames) => usernames.iter().fold(
                            column![text("Usernames:")]
                                .spacing(4)
                                .align_items(Alignment::Start),
                            |column: iced::widget::Column<'_, Message>, username| {
                                column.push(
                                    row![
                                        text(username),
                                        button("X").on_press(Message::RemoveUsername(
                                            username.to_string()
                                        ))
                                    ]
                                    .spacing(3)
                                    .align_items(Alignment::Center),
                                )
                            },
                        ),
                        Username::String(username) => column![
                            text("Libraries:"),
                            row![
                                button("X").on_press(Message::RemoveUsername(username.to_string())),
                                text(username),
                            ]
                            .spacing(3)
                            .align_items(Alignment::Center),
                        ]
                        .spacing(4)
                        .align_items(Alignment::Start),
                    };

                    column![back, add, usernames]
                        .spacing(10)
                        .align_items(Alignment::Center)
                }
                Setting::Images => {
                    let back = row![button("< Back")
                        .on_press(Message::Open(Panel::Settings(Setting::Main)))
                        .padding(5),]
                    .spacing(3)
                    .align_items(Alignment::Center);

                    let images =
                        checkbox("Enable Images", self.image_options.enabled, Message::Images);

                    let imgur = match self.image_options.enabled {
                        true => row![checkbox(
                            "Use Imgur",
                            self.image_options.imgur,
                            Message::Imgur
                        )],
                        false => row![],
                    };

                    let imgur_client_id = match self.image_options.imgur {
                        true => {
                            row![
                                text_input("abcdefg123456", &self.image_options.imgur_client_id)
                                    .on_input(Message::ImgurClientId)
                            ]
                        }
                        false => row![],
                    };

                    column![back, images, imgur, imgur_client_id]
                        .spacing(10)
                        .align_items(Alignment::Center)
                }
                Setting::Libraries => {
                    let back = row![button("< Back")
                        .on_press(Message::Open(Panel::Settings(Setting::Main)))
                        .padding(5),]
                    .spacing(3)
                    .align_items(Alignment::Center);

                    let libraries = self.libraries.iter().fold(
                        column![text("Libraries:")]
                            .spacing(4)
                            .align_items(Alignment::Start),
                        |column: iced::widget::Column<'_, Message>, library| {
                            column.push(
                                row![checkbox(&library.name, library.enabled, |val| {
                                    Message::ToggleLibrary(library.to_owned(), val)
                                }),]
                                .spacing(3)
                                .align_items(Alignment::Center),
                            )
                        },
                    );

                    column![back, libraries]
                        .spacing(10)
                        .align_items(Alignment::Center)
                }
            },
        };

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

pub struct Buttons {
    one: Button,
    two: Button,
}

impl Buttons {
    pub fn update(&mut self, new_buttons: Vec<Button>) {
        self.one = new_buttons[0].clone();
        self.two = new_buttons[1].clone();
    }
}

pub struct ImageOptions {
    enabled: bool,
    imgur: bool,
    imgur_client_id: String,
}

async fn get_libraries(url: String, api_key: String) -> Result<Vec<String>, reqwest::Error> {
    let media_folders: Value = serde_json::from_str(
        &reqwest::get(format!(
            "{}/Library/MediaFolders?api_key={}",
            url.trim_end_matches('/'),
            api_key
        ))
        .await?
        .text()
        .await?,
    )
    .unwrap();

    let items: Vec<Value> = media_folders["Items"].as_array().unwrap().to_vec();

    let mut libraries: Vec<String> = Vec::new();

    for library in items {
        let name = library["Name"].as_str().unwrap().to_string();
        libraries.push(name);
    }

    Ok(libraries)
}
