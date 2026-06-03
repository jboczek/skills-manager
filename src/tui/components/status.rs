use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = app
        .status_messages
        .iter()
        .map(|message| Line::from(Span::styled(message.clone(), Theme::muted())))
        .collect::<Vec<_>>();

    if let Some(message) = &app.error_message {
        lines.push(Line::from(Span::styled(message.clone(), Theme::error())));
    }
    if let Some(message) = &app.info_message {
        lines.push(Line::from(Span::styled(message.clone(), Theme::success())));
    }

    let max_lines = usize::from(area.height);
    if lines.len() > max_lines {
        lines = lines[lines.len().saturating_sub(max_lines)..].to_vec();
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), area);
}
