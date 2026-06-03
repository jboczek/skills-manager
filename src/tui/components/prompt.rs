use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::tui::app::{App, Mode};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let prompt_prefix = match app.mode {
        Mode::Import => format!("{} > ", app.import_step_hint()),
        Mode::Remove => format!("{} > ", app.remove_step_hint()),
        _ => "> ".to_string(),
    };
    let cursor = if app.mode == Mode::Quit { "" } else { "▏" };

    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(app.prompt_label.clone(), Theme::muted()),
            Line::styled(
                format!("{prompt_prefix}{}{cursor}", app.input),
                Theme::default_style(),
            ),
        ]),
        area,
    );
}
