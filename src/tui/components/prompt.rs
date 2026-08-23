use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table};

use crate::tui::app::{App, Mode};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    render_command_menu(frame, area, app);

    let prompt_prefix = match app.mode {
        Mode::SourceAdd => format!("{} > ", app.source_add_step_hint()),
        Mode::Import => format!("{} > ", app.import_step_hint()),
        Mode::Remove => format!("{} > ", app.remove_step_hint()),
        Mode::RepositoryUpdate => format!("{} > ", app.repository_update_step_hint()),
        _ => "> ".to_string(),
    };
    let cursor = if app.mode == Mode::Quit { "" } else { "▏" };

    frame.render_widget(
        Paragraph::new(vec![
            prompt_title(app),
            Line::styled(
                format!("{prompt_prefix}{}{cursor}", app.input),
                Theme::default_style(),
            ),
        ]),
        area,
    );
}

fn prompt_title(app: &App) -> Line<'static> {
    let mut spans = vec![Span::styled(app.prompt_label.clone(), Theme::muted())];
    if app.mode == Mode::List {
        spans.push(Span::styled(" · ", Theme::muted()));
        spans.push(Span::styled(app.list_filter.label(), Theme::header()));
    }
    Line::from(spans)
}

fn render_command_menu(frame: &mut Frame, prompt_area: Rect, app: &App) {
    if !app.command_menu_open() {
        return;
    }

    let suggestions = app.filtered_command_suggestions();
    let height = u16::try_from(suggestions.len().max(1) + 2)
        .unwrap_or(7)
        .min(7);
    let area = Rect {
        x: prompt_area.x,
        y: prompt_area.y.saturating_sub(height),
        width: prompt_area.width,
        height,
    };

    frame.render_widget(Clear, area);

    if suggestions.is_empty() {
        frame.render_widget(
            Paragraph::new("No matching commands.")
                .block(menu_block())
                .style(Theme::muted()),
            area,
        );
        return;
    }

    let selected = app
        .selected_command_suggestion()
        .map(|suggestion| suggestion.label);
    let rows = suggestions
        .iter()
        .map(|suggestion| {
            let row = Row::new(vec![
                Cell::from(Line::from(Span::styled(suggestion.label, Theme::accent2()))),
                Cell::from(suggestion.description),
            ]);
            if selected == Some(suggestion.label) {
                row.style(Theme::selected())
            } else {
                row.style(Theme::default_style())
            }
        })
        .collect::<Vec<_>>();

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(20)])
        .block(menu_block())
        .column_spacing(1)
        .style(Theme::default_style());

    frame.render_widget(table, area);
}

fn menu_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::border())
        .title(" Commands ")
        .title_style(Theme::header())
        .style(Theme::default_style())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::unified_list::ListFilter;

    #[test]
    fn list_filter_labels_are_human_readable() {
        assert_eq!(ListFilter::Full.label(), "Full");
        assert_eq!(ListFilter::OnlyExposed.label(), "Only exposed");
        assert_eq!(
            ListFilter::OnlyDiscovered.label(),
            "Only discovered not applied"
        );
    }

    #[test]
    fn list_prompt_shows_the_active_filter_in_the_accent_color() {
        let mut app = App::new(Config::default_config()).unwrap();
        app.mode = Mode::List;
        app.list_filter = ListFilter::OnlyExposed;

        let title = prompt_title(&app);

        assert_eq!(title.spans.len(), 3);
        assert_eq!(title.spans[0].content.as_ref(), "Skills");
        assert_eq!(title.spans[1].content.as_ref(), " · ");
        assert_eq!(title.spans[2].content.as_ref(), "Only exposed");
        assert_eq!(title.spans[2].style.fg, Some(Theme::ACCENT));
    }
}
