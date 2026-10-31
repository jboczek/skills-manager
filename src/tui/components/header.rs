use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .title(" Skills Manager v0.1 ")
        .title_style(Theme::header())
        .style(Theme::default_style());
    let inner = block.inner(area);

    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(Text::from(header_text(app.available_update.as_deref())))
            .style(Theme::muted()),
        inner,
    );
}

fn header_text(available_update: Option<&str>) -> String {
    let mut text = "Manage global and project-local skills for Codex, Claude and Copilot\nType a command or use /help to get started".to_string();
    if let Some(version) = available_update {
        text.push_str(&format!("\nNew version of Skills Manager {version} is available. Type /update to update to the newest version."));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::header_text;

    #[test]
    fn shows_update_notice_below_startup_guidance() {
        assert!(header_text(Some("0.2.0")).ends_with(
            "New version of Skills Manager 0.2.0 is available. Type /update to update to the newest version."
        ));
    }
}
