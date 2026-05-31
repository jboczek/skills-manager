use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub const BACKGROUND: Color = Color::Rgb(10, 12, 20);
    pub const TEXT: Color = Color::Rgb(200, 200, 210);
    pub const MUTED: Color = Color::Rgb(110, 110, 130);
    pub const ACCENT: Color = Color::Rgb(140, 100, 220);
    pub const ACCENT2: Color = Color::Rgb(80, 200, 200);
    pub const WARNING: Color = Color::Rgb(220, 180, 50);
    pub const ERROR: Color = Color::Rgb(220, 80, 80);
    pub const SUCCESS: Color = Color::Rgb(80, 200, 120);
    pub const BORDER: Color = Color::Rgb(50, 55, 75);

    pub fn default_style() -> Style {
        Style::default().fg(Self::TEXT).bg(Self::BACKGROUND)
    }

    pub fn muted() -> Style {
        Style::default().fg(Self::MUTED).bg(Self::BACKGROUND)
    }

    pub fn accent() -> Style {
        Style::default().fg(Self::ACCENT).bg(Self::BACKGROUND)
    }

    pub fn accent2() -> Style {
        Style::default().fg(Self::ACCENT2).bg(Self::BACKGROUND)
    }

    pub fn warning() -> Style {
        Style::default().fg(Self::WARNING).bg(Self::BACKGROUND)
    }

    pub fn error() -> Style {
        Style::default().fg(Self::ERROR).bg(Self::BACKGROUND)
    }

    pub fn success() -> Style {
        Style::default().fg(Self::SUCCESS).bg(Self::BACKGROUND)
    }

    pub fn selected() -> Style {
        Style::default().fg(Self::BACKGROUND).bg(Self::ACCENT)
    }

    pub fn border() -> Style {
        Style::default().fg(Self::BORDER).bg(Self::BACKGROUND)
    }

    pub fn header() -> Style {
        Style::default()
            .fg(Self::ACCENT)
            .bg(Self::BACKGROUND)
            .add_modifier(Modifier::BOLD)
    }
}
