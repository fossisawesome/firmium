use ratatui::{
    widgets::{Block, Borders, Gauge, Paragraph},
    layout::{Constraint, Direction, Layout},
    Frame,
};
use crate::app::App;

pub fn render(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(50), Constraint::Percentage(10)])
        .split(area);

    let title = match &app.now_playing {
        Some(song) => format!("{} — {}", song.title, song.artist),
        None => "Nothing playing".to_string(),
    };
    frame.render_widget(
        Paragraph::new(title).block(Block::default().borders(Borders::ALL)),
        chunks[0],
    );

    let ratio = match app.playback_duration {
        Some(d) if d > 0.0 => (app.playback_position / d).clamp(0.0, 1.0),
        _ => 0.0,
    };
    frame.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .ratio(ratio)
            .label(if app.is_playing { "Playing" } else { "Paused" }),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(format!("Vol {}%", (app.volume * 100.0) as u32))
            .block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}
