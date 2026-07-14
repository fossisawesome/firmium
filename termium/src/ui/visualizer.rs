use ratatui::{widgets::Paragraph, Frame};
use crate::app::App;

const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub fn render(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let width = area.width as usize;
    let samples = &app.viz_snapshot;
    if samples.is_empty() {
        frame.render_widget(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL), area);
        return;
    }

    let step = (samples.len() / width.max(1)).max(1);
    let max = samples.iter().cloned().fold(0.0_f32, f32::max).max(0.001);
    let line: String = samples
        .iter()
        .step_by(step)
        .take(width)
        .map(|&v| {
            let magnitude = (v / max).clamp(0.0, 1.0);
            let idx = ((magnitude * (BARS.len() - 1) as f32).round() as usize).min(BARS.len() - 1);
            BARS[idx]
        })
        .collect();

    frame.render_widget(Paragraph::new(line), area);
}
