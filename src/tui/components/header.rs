use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .title(" Skills Manager v0.1 ")
        .title_style(Theme::header())
        .style(Theme::default_style());
    let inner = block.inner(area);

    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(Text::from(
            "Manage global and project-local skills for Codex, Claude and Copilot\nType a command or use /help to get started",
        ))
        .style(Theme::muted()),
        inner,
    );
}
