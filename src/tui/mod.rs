pub mod app;
pub mod components;
pub mod events;
pub mod layout;
pub mod source_table;
pub mod theme;

#[cfg(test)]
mod mod_tests;

fn configure_mouse_capture<W: std::io::Write>(
    output: &mut W,
    enabled: bool,
) -> std::io::Result<()> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};

    if enabled {
        crossterm::execute!(output, EnableMouseCapture)
    } else {
        crossterm::execute!(output, DisableMouseCapture)
    }
}

pub fn run() -> anyhow::Result<()> {
    use crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    };
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;
    use std::io;
    use std::io::IsTerminal;

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    let config = match crate::config::Config::default_path() {
        Some(path) if path.exists() => crate::config::Config::load_from(&path)?,
        _ => crate::config::Config::default_config(),
    };
    let mut app = app::App::new(config)?;

    #[derive(Default)]
    struct TerminalCleanup {
        raw_mode_enabled: bool,
        alternate_screen_enabled: bool,
        mouse_capture_enabled: bool,
    }

    impl Drop for TerminalCleanup {
        fn drop(&mut self) {
            if self.mouse_capture_enabled {
                let mut stdout = io::stdout();
                let _ = configure_mouse_capture(&mut stdout, false);
            }
            if self.raw_mode_enabled {
                let _ = disable_raw_mode();
            }
            if self.alternate_screen_enabled {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen);
            }
        }
    }

    fn cleanup_terminal<B: ratatui::backend::Backend>(
        terminal: &mut ratatui::Terminal<B>,
        cleanup: &mut TerminalCleanup,
    ) -> anyhow::Result<()> {
        if cleanup.mouse_capture_enabled {
            let mut stdout = io::stdout();
            configure_mouse_capture(&mut stdout, false)?;
            cleanup.mouse_capture_enabled = false;
        }
        if cleanup.raw_mode_enabled {
            disable_raw_mode()?;
            cleanup.raw_mode_enabled = false;
        }
        if cleanup.alternate_screen_enabled {
            let mut stdout = io::stdout();
            execute!(stdout, LeaveAlternateScreen)?;
            cleanup.alternate_screen_enabled = false;
        }
        terminal
            .show_cursor()
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        Ok(())
    }

    enable_raw_mode()?;
    let mut cleanup = TerminalCleanup {
        raw_mode_enabled: true,
        alternate_screen_enabled: false,
        mouse_capture_enabled: false,
    };

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    cleanup.alternate_screen_enabled = true;
    cleanup.mouse_capture_enabled = true;
    configure_mouse_capture(&mut stdout, true)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    app.start_initial_load();

    let result = event_loop(&mut terminal, &mut app);
    let cleanup_result = cleanup_terminal(&mut terminal, &mut cleanup);

    result.and(cleanup_result)
}

fn event_loop<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut app::App,
) -> anyhow::Result<()> {
    loop {
        terminal
            .draw(|frame| draw(frame, app))
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        app.poll_initial_load();

        if app.pending_load.is_some() {
            if let Err(error) = app.execute_pending_load() {
                app.loading = false;
                app.error_message = Some(error.to_string());
            }
            continue;
        }

        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) if events::handle_key(app, key)? => break,
                crossterm::event::Event::Resize(width, height) => {
                    let layout = layout::AppLayout::compute(ratatui::layout::Rect {
                        x: 0,
                        y: 0,
                        width,
                        height,
                    });
                    app.sync_active_table(events::table_height_for_main(layout.main.height));
                }
                _ => {}
            }
        }

        if app.mode == app::Mode::Quit {
            break;
        }
    }
    Ok(())
}

fn draw(frame: &mut ratatui::Frame, app: &app::App) {
    use layout::AppLayout;

    let layout = AppLayout::compute(frame.area());

    frame.render_widget(
        ratatui::widgets::Block::default().style(theme::Theme::default_style()),
        frame.area(),
    );

    components::header::render(frame, layout.header, app);
    components::status::render(frame, layout.status, app);
    components::main_panel::render(frame, layout.main, app);
    components::prompt::render(frame, layout.prompt, app);
    components::footer::render(frame, layout.footer, app);
}
