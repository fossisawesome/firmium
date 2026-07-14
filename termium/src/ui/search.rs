use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    frame.render_widget(
        Paragraph::new(app.search_query.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search (Enter to run, Esc to cancel)")),
        chunks[0],
    );

    let items: Vec<ListItem> = app
        .search_results_songs
        .iter()
        .map(|s| ListItem::new(format!("{} — {} ({})", s.title, s.artist, s.album)))
        .collect();
    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Results"))
        .highlight_style(ratatui::style::Style::new().reversed());
    frame.render_stateful_widget(list, chunks[1], &mut state);
}
