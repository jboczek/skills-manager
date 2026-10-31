use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::app::App;
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let width = usize::from(area.width).max(1);
    let mut lines = Vec::new();
    for message in &app.status_messages {
        push_wrapped_message(&mut lines, message, Theme::muted(), width);
    }

    if let Some(message) = &app.error_message {
        push_wrapped_message(&mut lines, message, Theme::error(), width);
    }
    if let Some(message) = &app.info_message {
        push_wrapped_message(&mut lines, message, Theme::success(), width);
    }

    let max_lines = usize::from(area.height);
    if lines.len() > max_lines {
        lines = lines[lines.len().saturating_sub(max_lines)..].to_vec();
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), area);
}

fn push_wrapped_message(lines: &mut Vec<Line<'static>>, message: &str, style: Style, width: usize) {
    for line in wrap_message(message, width) {
        lines.push(Line::from(Span::styled(line, style)));
    }
}

fn wrap_message(message: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut wrapped = Vec::new();

    for source_line in message.lines() {
        if source_line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in source_line.split_whitespace() {
            push_word(&mut wrapped, &mut current, word, width);
        }
        if !current.is_empty() {
            wrapped.push(current);
        }
    }

    wrapped
}

fn push_word(wrapped: &mut Vec<String>, current: &mut String, word: &str, width: usize) {
    let separator_width = usize::from(!current.is_empty());
    if visible_width(current) + separator_width + visible_width(word) <= width {
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
        return;
    }

    if !current.is_empty() {
        wrapped.push(std::mem::take(current));
    }
    push_word_chunks(wrapped, current, word, width);
}

fn push_word_chunks(wrapped: &mut Vec<String>, current: &mut String, word: &str, width: usize) {
    let mut chunk = String::new();
    for character in word.chars() {
        if visible_width(&chunk) == width {
            wrapped.push(std::mem::take(&mut chunk));
        }
        chunk.push(character);
    }
    *current = chunk;
}

fn visible_width(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::config::Config;

    fn rendered_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn long_error_message_wraps_in_narrow_status_area() {
        let mut app = App::new(Config::default_config()).expect("default config resolves");
        app.status_messages.clear();
        app.error_message = Some(
            "Repository URL must be the HTTPS or SSH clone URL from the repository page."
                .to_string(),
        );
        let backend = TestBackend::new(32, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render(frame, frame.area(), &app))
            .unwrap();

        let text = rendered_lines(&terminal).join("\n");
        assert!(text.contains("SSH clone URL"));
        assert!(text.contains("repository page."));
    }

    #[test]
    fn reference_width_keeps_all_four_status_lines_visible() {
        let mut app = App::new(Config::default_config()).expect("default config resolves");
        app.status_messages = vec![
            "• Global context: 5 skills, 3 agents".to_string(),
            "• Estimated tokens (global): Claude ~776 (1 skill) · Codex ~6394 (2 skills) · Copilot ~1568 (0 skills)".to_string(),
            "• Scan: OK".to_string(),
            "• Imported: 5 · Discovered not imported: 1".to_string(),
        ];
        let backend = TestBackend::new(160, 4);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| render(frame, frame.area(), &app))
            .unwrap();

        let lines = rendered_lines(&terminal);
        assert!(lines[0].contains("Global context: 5 skills, 3 agents"));
        assert!(lines[1].contains("Estimated tokens (global):"));
        assert!(lines[1].contains("~776 (1 skill)"));
        assert!(lines[1].contains("~6394 (2 skills)"));
        assert!(lines[1].contains("~1568 (0 skills)"));
        assert!(lines[2].contains("Scan: OK"));
        assert!(lines[3].contains("Imported: 5 · Discovered not imported: 1"));
    }
}
