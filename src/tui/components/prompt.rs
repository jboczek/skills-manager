use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table};

use crate::tui::app::{App, Mode};
use crate::tui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    render_command_menu(frame, area, app);

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
