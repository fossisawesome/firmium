use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use crate::app::{App, View};

mod login;
mod home;
mod albums;
mod artists;
mod search;
mod playlists;
mod player_bar;
mod visualizer;

pub fn render(app: &App, frame: &mut Frame) {
    if app.backend.is_none() {
        login::render(app, frame);
        return;
    }

    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3), Constraint::Length(3)])
        .split(area);

    match app.view {
        View::Login => login::render(app, frame),
        View::Home => home::render(app, frame, chunks[0]),
        View::Albums => albums::render(app, frame, chunks[0]),
        View::Artists => artists::render(app, frame, chunks[0]),
        View::Search => search::render(app, frame, chunks[0]),
        View::Playlists => playlists::render(app, frame, chunks[0]),
    }

    visualizer::render(app, frame, chunks[1]);
    player_bar::render(app, frame, chunks[2]);
}
