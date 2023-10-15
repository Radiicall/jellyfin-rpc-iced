use iced::{executor, Length, Alignment};
use iced::widget::{container, button, Column, text};
use iced::{Application, Command, Element, Theme};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Message {
    Update,
    Start,
    Stop,
    Recieve
}

#[derive(Default)]
pub struct Data {
    pub rx: Option<mpsc::Receiver<String>>,
    pub tx: Option<mpsc::Sender<String>>
}

pub struct Gui {
    status: String,
    rx: mpsc::Receiver<String>,
    tx: mpsc::Sender<String>,
}

impl Application for Gui {
    type Executor = executor::Default;
    type Flags = Data;
    type Message = Message;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Gui, Command<Message>) {
        (
            Gui { status: "Unknown".to_string(), rx: flags.rx.unwrap(), tx: flags.tx.unwrap() },
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("A cool application")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Update => {
                self.tx.send("reload_config".to_string()).unwrap()
            },
            Message::Start => {
                self.tx.send("start".to_string()).unwrap()
            },
            Message::Stop => {
                self.tx.send("stop".to_string()).unwrap()
            }
            Message::Recieve => {
                let new = self.rx.try_recv().unwrap_or_else(|_| "".to_string());
                if new != "" {
                    self.status = new;
                }
            }
        };
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| {
            Message::Recieve
        })
    }

    fn view(&self) -> Element<Message> {
        let downloads = Column::with_children(
            vec![Column::new()
                .spacing(10)
                .padding(10)
                .align_items(Alignment::Center)
                .into()],
        )
        .push(
            text(self.status.clone())
        )
        .push(
            button("Gaming")
                .on_press(Message::Update)
                .padding(10),
        )
        .push(
            button("Start")
                .on_press(Message::Start)
                .padding(10),
        )
        .push(
            button("Stop")
                .on_press(Message::Stop)
                .padding(10),
        )
        .spacing(20)
        .align_items(Alignment::End);

        container(downloads)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(20)
            .into()
    }
}
