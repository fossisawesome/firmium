use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;

pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let field_style = |idx: u8| {
        if app.login_field_focus == idx {
            Style::new().bold()
        } else {
            Style::new()
        }
    };

    frame.render_widget(
        Paragraph::new(app.login_server.as_str())
            .block(Block::default().borders(Borders::ALL).title("Server"))
            .style(field_style(0)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(app.login_username.as_str())
            .block(Block::default().borders(Borders::ALL).title("Username"))
            .style(field_style(1)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("*".repeat(app.login_password.len()))
            .block(Block::default().borders(Borders::ALL).title("Password"))
            .style(field_style(2)),
        chunks[2],
    );
    let help = app
        .status_message
        .clone()
        .unwrap_or_else(|| "Tab: next field   Enter: log in   q: quit".to_string());
    frame.render_widget(Paragraph::new(help), chunks[3]);
}
