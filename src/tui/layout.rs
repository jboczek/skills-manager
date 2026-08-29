use ratatui::layout::{Constraint, Direction, Layout as LayoutEngine, Rect};

pub struct AppLayout {
    pub header: Rect,
    pub status: Rect,
    pub main: Rect,
    pub prompt: Rect,
    pub footer: Rect,
}

impl AppLayout {
    /// Split the terminal area into the 5 fixed regions.
    pub fn compute(area: Rect) -> Self {
        let regions = LayoutEngine::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(area);

        Self {
            header: regions[0],
            status: regions[1],
            main: regions[2],
            prompt: regions[3],
            footer: regions[4],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_reserves_three_content_lines_for_an_update_notice() {
        assert_eq!(
            AppLayout::compute(Rect::new(0, 0, 120, 30)).header.height,
            5
        );
    }
}
