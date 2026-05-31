use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::theme::Theme;

/// Render a confirmation dialog box in the center of the screen.
pub fn render_confirm_dialog(
    frame: &mut Frame,
    title: &str,
    body: &str,
    is_destructive: bool,
) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(frame.area());
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(vertical[1]);
    let area: Rect = horizontal[1];
    let title = if is_destructive {
        format!("⚠ {title}")
    } else {
        title.to_string()
    };

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if is_destructive {
                        Theme::warning()
                    } else {
                        Theme::accent()
                    })
                    .title(title)
                    .title_style(if is_destructive {
                        Theme::warning()
                    } else {
                        Theme::header()
                    })
                    .style(Theme::default_style()),
            )
            .style(if is_destructive {
                Theme::warning()
            } else {
                Theme::default_style()
            })
            .wrap(Wrap { trim: false }),
        area,
    );
}
