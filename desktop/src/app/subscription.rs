use std::hash::{Hash, Hasher};

use iced::Subscription;

use firmium_backend::events::EventBus;

use super::message::Message;
use super::types::Panel;
use super::App;

impl App {
    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let bus = Subscription::run_with(BusSub(self.backend.bus.clone()), bus_stream);
        Subscription::batch([
            bus,
            if self.right_panel == Some(Panel::Visualizer) {
                iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::VisualizerTick)
            } else {
                Subscription::none()
            },
            if self.toasts.is_empty() {
                Subscription::none()
            } else {
                iced::time::every(std::time::Duration::from_millis(500)).map(|_| Message::ToastTick)
            },
        ])
    }
}

pub(crate) struct BusSub(EventBus);

impl Hash for BusSub {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "firmium-event-bus".hash(state);
    }
}

pub(crate) fn bus_stream(data: &BusSub) -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::SinkExt;
    use tokio::sync::broadcast::error::RecvError;

    let bus = data.0.clone();
    iced::stream::channel(64, move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        let mut rx = bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let _ = output.send(Message::Backend(event)).await;
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    })
}
