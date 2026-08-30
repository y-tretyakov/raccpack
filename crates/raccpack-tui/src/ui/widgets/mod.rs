//! Reusable widgets shared across screens (detail strip, …).

pub mod detail;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Compute a centered rectangle as a percentage of the parent.
///
/// Shared by overlays (help, raid modal) so every popup centers the same way.
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_is_smaller_than_parent() {
        let parent = Rect::new(0, 0, 100, 50);
        let child = centered_rect(60, 40, parent);
        assert!(child.width < parent.width);
        assert!(child.height < parent.height);
    }

    #[test]
    fn centered_rect_has_positive_dimensions() {
        let parent = Rect::new(0, 0, 80, 24);
        let child = centered_rect(60, 85, parent);
        assert!(child.width > 0);
        assert!(child.height > 0);
    }

    #[test]
    fn centered_rect_is_centered() {
        let parent = Rect::new(0, 0, 100, 50);
        let child = centered_rect(60, 40, parent);
        let left_margin = child.x.saturating_sub(parent.x);
        let right_margin = (parent.x + parent.width).saturating_sub(child.x + child.width);
        // Margins should be equal (±1 for rounding)
        assert!((left_margin as i16 - right_margin as i16).unsigned_abs() <= 1);
    }
}
