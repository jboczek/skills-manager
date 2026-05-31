use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::app::{App, ImportStep, Mode, RemoveStep};

/// Handle a key event. Returns true if the app should quit.
pub fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    match key.code {
        KeyCode::Esc => {
            app.input.clear();
            app.mode = Mode::Home;
            app.import_step = ImportStep::EnterSkill;
            app.remove_step = RemoveStep::EnterSkill;
            app.error_message = None;
            app.info_message = None;
        }
        KeyCode::Enter => {
            let input = app.input.clone();
            let should_quit = app.handle_command(&input)?;
            app.input.clear();
            return Ok(should_quit);
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Up if app.mode == Mode::List => {
            if app.inventory.is_empty() {
                return Ok(false);
            }
            let next = app.list_selected.unwrap_or(0).saturating_sub(1);
            app.list_selected = Some(next);
            app.list_scroll = next;
        }
        KeyCode::Down if app.mode == Mode::List => {
            if app.inventory.is_empty() {
                return Ok(false);
            }
            let next = app
                .list_selected
                .map(|selected| selected.saturating_add(1).min(app.inventory.len() - 1))
                .unwrap_or(0);
            app.list_selected = Some(next);
            app.list_scroll = next;
        }
        KeyCode::Char('?') if app.mode == Mode::Home && app.input.is_empty() => {
            app.mode = Mode::Help;
        }
        KeyCode::Char('q') if app.mode == Mode::Home && app.input.is_empty() => {
            return Ok(true);
        }
        KeyCode::Char(c) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            app.input.push(c);
        }
        _ => {}
    }

    Ok(false)
}
