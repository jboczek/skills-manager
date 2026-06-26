use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

use crate::tui::app::{App, Mode};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let hint = match app.mode {
        Mode::Home => "/ commands   ? help   q quit",
        Mode::List => {
            "arrows browse/expand   space check   i import   x remove   r refresh   esc back   q quit"
        }
        Mode::Scan => "arrows browse/expand   i import skill   r refresh   esc back   q quit",
        Mode::SourceAdd | Mode::Import | Mode::Remove => "enter confirm   esc cancel",
        Mode::Help => "esc back",
        Mode::Config => "esc back   q quit",
        Mode::Quit => "quitting...",
    };

    frame.render_widget(Paragraph::new(hint).style(Theme::muted()), area);
}
