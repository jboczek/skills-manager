pub mod app;
pub mod components;
pub mod events;
pub mod layout;
pub mod theme;

pub fn run() -> anyhow::Result<()> {
    use crossterm::{
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
    use std::io;
    use std::io::IsTerminal;

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    #[derive(Default)]
    struct TerminalCleanup {
        raw_mode_enabled: bool,
        alternate_screen_enabled: bool,
    }

    impl Drop for TerminalCleanup {
        fn drop(&mut self) {
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
    };

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    cleanup.alternate_screen_enabled = true;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = crate::config::Config::default_path()
        .and_then(|path| {
            path.exists()
                .then(|| crate::config::Config::load_from(&path).ok())
                .flatten()
        })
        .unwrap_or_else(crate::config::Config::default_config);
    let current_dir =
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut app = app::App::new(config, current_dir);
    if let Err(error) = app.initialize() {
        app.error_message = Some(error.to_string());
    }

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

        if crossterm::event::poll(std::time::Duration::from_millis(50))?
            && let crossterm::event::Event::Key(key) = crossterm::event::read()?
            && events::handle_key(app, key)?
        {
            break;
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
