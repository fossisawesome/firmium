use ratatui::{
    style::{Style, Stylize},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .albums
        .iter()
        .map(|a| ListItem::new(format!("{} — {}", a.name, a.album_artist)))
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Albums"))
        .highlight_style(Style::new().reversed());
    frame.render_stateful_widget(list, area, &mut state);
}
