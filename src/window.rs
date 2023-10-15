use iced::widget::{button, container, text, Column, Row};
use iced::{executor, Alignment, Length};
use iced::{Application, Command, Element, Theme};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Message {
    ReloadConfig,
    Start,
    Stop,
    Recieve,
}

#[derive(Default)]
pub struct Data {
    pub rx: Option<mpsc::Receiver<String>>,
    pub error_rx: Option<mpsc::Receiver<String>>,
    pub tx: Option<mpsc::Sender<String>>,
}

pub struct Gui {
    status: String,
    error: String,
    rx: mpsc::Receiver<String>,
    error_rx: mpsc::Receiver<String>,
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
                rx: flags.rx.unwrap(),
                error_rx: flags.error_rx.unwrap(),
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
            Message::Recieve => {
                let new_status = self.rx.try_recv().unwrap_or_else(|_| "".to_string());
                if !new_status.is_empty() {
                    self.status = new_status;
                }

                let new_error = self.error_rx.try_recv().unwrap_or_else(|_| "".to_string());
                if !new_error.is_empty() {
                    self.error = new_error;
                }
            }
        };
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Recieve)
    }

    fn view(&self) -> Element<Message> {
        let view = Column::new()
            .push(
                Row::new()
                    .push(button("Start").on_press(Message::Start).padding(10))
                    .push(button("Stop").on_press(Message::Stop).padding(10))
                    .spacing(10)
                    .align_items(Alignment::Center),
            )
            .push(
                button("Reload Config")
                    .on_press(Message::ReloadConfig)
                    .padding(10),
            )
            .push(
                Column::new()
                    .push(text("Status: ").size(30))
                    .push(text(self.status.clone()))
                    .align_items(Alignment::Center),
            )
            .push(
                Column::new()
                    .push(text("Error: ").size(30))
                    .push(text(self.error.clone()))
                    .spacing(10)
                    .align_items(Alignment::Center),
            )
            .spacing(10)
            .align_items(Alignment::Center);

        container(view)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .padding(20)
            .into()
    }
}
