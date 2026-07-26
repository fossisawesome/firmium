use std::hash::{Hash, Hasher};

use iced::{event, keyboard, mouse, window};
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
            keyboard::listen().filter_map(key_message),
            event::listen_with(mouse_message),
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

// `Subscription::filter_map` requires a non-capturing closure, so overlay/focus
// state can't be read here — Escape always dispatches `EscapePressed`, which
// `update_nav` resolves against current app state to close whichever overlay is
// on top. Space maps straight to `TogglePlay`; since `keyboard::listen()` only
// yields events with `Status::Ignored` (not consumed by a focused widget), a
// focused text input's own key handling naturally shadows this, so typing a
// space in the search box doesn't also toggle playback.
fn key_message(event: keyboard::Event) -> Option<Message> {
    match event {
        keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Escape), .. } => {
            Some(Message::EscapePressed)
        }
        keyboard::Event::KeyPressed { key: keyboard::Key::Named(keyboard::key::Named::Space), .. } => {
            Some(Message::TogglePlay)
        }
        _ => None,
    }
}

fn mouse_message(event: iced::Event, status: event::Status, _window: window::Id) -> Option<Message> {
    match (event, status) {
        (iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Back)), event::Status::Ignored) => {
            Some(Message::NavigateBack)
        }
        (iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Forward)), event::Status::Ignored) => {
            Some(Message::NavigateForward)
        }
        _ => None,
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
