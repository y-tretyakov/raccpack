use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::nowbar::{NowPlaying, TimeProvider};

// ---------------------------------------------------------------------------
// FocusGrid
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct FocusCell {
    pub focused: bool,
    pub app_label: Option<String>,
}

pub struct FocusGrid {
    pub cells: Vec<Vec<FocusCell>>,
    pub active_row: usize,
    pub active_col: usize,
}

impl Default for FocusGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusGrid {
    pub fn new() -> Self {
        Self {
            cells: vec![
                vec![
                    FocusCell {
                        focused: false,
                        app_label: None,
                    };
                    3
                ];
                2
            ],
            active_row: 0,
            active_col: 0,
        }
    }

    pub fn set_cell(&mut self, row: usize, col: usize, label: Option<String>) {
        if row < self.cells.len() && col < self.cells[row].len() {
            self.cells[row][col].app_label = label;
        }
    }

    pub fn set_focus(&mut self, row: usize, col: usize) {
        for r in &mut self.cells {
            for c in r {
                c.focused = false;
            }
        }
        if row < self.cells.len() && col < self.cells[row].len() {
            self.cells[row][col].focused = true;
            self.active_row = row;
            self.active_col = col;
        }
    }

    pub fn focused_label(&self) -> Option<&str> {
        self.cells
            .get(self.active_row)?
            .get(self.active_col)?
            .app_label
            .as_deref()
    }
}

impl Widget for &FocusGrid {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let rows = self.cells.len() as u16;
        let cols = self.cells.first().map_or(0, |r| r.len() as u16);
        if rows == 0 || cols == 0 {
            return;
        }
        let cell_w = area.width / cols;
        let cell_h = area.height / rows;

        for (r, row) in self.cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let x = area.x + (c as u16) * cell_w;
                let y = area.y + (r as u16) * cell_h;
                let style = if cell.focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let label = cell.app_label.as_deref().unwrap_or("·");
                buf.set_string(x, y, label, style);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MonitorDock
// ---------------------------------------------------------------------------

pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub focused: bool,
}

pub struct MonitorDock<'a> {
    pub monitors: &'a [MonitorInfo],
    pub focused_monitor: usize,
}

impl<'a> Widget for MonitorDock<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut spans = Vec::new();
        for (i, m) in self.monitors.iter().enumerate() {
            let style = if i == self.focused_monitor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            spans.push(Span::styled(format!(" M{} ", m.id), style));
            let _ = m.name;
        }
        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

// ---------------------------------------------------------------------------
// AppsList
// ---------------------------------------------------------------------------

pub struct AppItem {
    pub name: String,
    pub monitor: u32,
    pub workspace: String,
    pub pinned: bool,
}

pub struct AppsList<'a> {
    pub items: &'a [AppItem],
    pub selected: usize,
    pub scroll_offset: usize,
}

impl<'a> Widget for AppsList<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let visible = area.height as usize;
        for i in 0..visible.min(self.items.len()) {
            let idx = i + self.scroll_offset.min(self.items.len());
            if idx >= self.items.len() {
                break;
            }
            let item = &self.items[idx];
            let y = area.y + i as u16;
            let style = if idx == self.selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let prefix = if item.pinned { "pin " } else { "    " };
            let label = format!("{}{} [M{}]", prefix, item.name, item.monitor);
            buf.set_string(area.x, y, &label, style);
        }
    }
}

// ---------------------------------------------------------------------------
// ContextBar
// ---------------------------------------------------------------------------

pub struct ContextItem {
    pub key: char,
    pub label: String,
    pub enabled: bool,
}

pub struct ContextBar<'a> {
    pub items: &'a [ContextItem],
}

impl<'a> Widget for ContextBar<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut spans = Vec::new();
        for item in self.items {
            let style = if item.enabled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Gray)
            };
            spans.push(Span::styled(
                format!(" [{}] {} ", item.key, item.label),
                style,
            ));
        }
        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

// ---------------------------------------------------------------------------
// NotificationBar
// ---------------------------------------------------------------------------

pub enum NotificationKind {
    Info,
    Success,
    Error,
    TrackChange,
}

pub struct NotificationBar<'a> {
    pub message: Option<&'a str>,
    pub kind: NotificationKind,
}

impl<'a> Widget for NotificationBar<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let msg = self.message.unwrap_or("");
        let style = match self.kind {
            NotificationKind::Info => Style::default().fg(Color::Gray),
            NotificationKind::Success => Style::default().fg(Color::Green),
            NotificationKind::Error => Style::default().fg(Color::Red),
            NotificationKind::TrackChange => Style::default().fg(Color::Cyan),
        };
        buf.set_string(area.x, area.y, msg, style);
    }
}

// ---------------------------------------------------------------------------
// NowBarWidget
// ---------------------------------------------------------------------------

pub struct NowBarWidget<'a> {
    pub time: &'a TimeProvider,
    pub track: &'a NowPlaying,
}

impl<'a> Widget for NowBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let time_str = self.time.time_24h();
        let track_str = self
            .track
            .track_title()
            .map(|t| format!(" | \u{266A} {}", t))
            .unwrap_or_default();
        let content = format!(" {}{}", time_str, track_str);
        buf.set_string(area.x, area.y, &content, Style::default().fg(Color::Gray));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    fn area(w: u16, h: u16) -> Rect {
        Rect::new(0, 0, w, h)
    }

    // -- FocusGrid --

    #[test]
    fn focus_grid_new_has_default_dimensions() {
        let grid = FocusGrid::new();
        assert_eq!(grid.cells.len(), 2);
        assert_eq!(grid.cells[0].len(), 3);
    }

    #[test]
    fn focus_grid_set_cell_and_read() {
        let mut grid = FocusGrid::new();
        grid.set_cell(0, 1, Some("browser".into()));
        assert_eq!(grid.cells[0][1].app_label.as_deref(), Some("browser"));
    }

    #[test]
    fn focus_grid_set_cell_out_of_bounds_is_noop() {
        let mut grid = FocusGrid::new();
        grid.set_cell(5, 5, Some("x".into()));
        assert!(grid.cells.get(5).is_none());
    }

    #[test]
    fn focus_grid_set_focus_moves_active() {
        let mut grid = FocusGrid::new();
        grid.set_focus(1, 2);
        assert!(grid.cells[1][2].focused);
        assert_eq!(grid.active_row, 1);
        assert_eq!(grid.active_col, 2);
    }

    #[test]
    fn focus_grid_set_focus_clears_previous() {
        let mut grid = FocusGrid::new();
        grid.set_focus(0, 0);
        assert!(grid.cells[0][0].focused);
        grid.set_focus(1, 1);
        assert!(!grid.cells[0][0].focused);
    }

    #[test]
    fn focus_grid_focused_label_returns_active_cell() {
        let mut grid = FocusGrid::new();
        grid.set_cell(1, 0, Some("editor".into()));
        grid.set_focus(1, 0);
        assert_eq!(grid.focused_label(), Some("editor"));
    }

    #[test]
    fn focus_grid_focused_label_none_when_empty() {
        let grid = FocusGrid::new();
        assert_eq!(grid.focused_label(), None);
    }

    #[test]
    fn focus_grid_render_does_not_panic() {
        let grid = FocusGrid::new();
        let mut buf = Buffer::empty(area(30, 10));
        Widget::render(&grid, area(30, 10), &mut buf);
    }

    // -- AppsList --

    #[test]
    fn apps_list_render_does_not_panic() {
        let items = vec![
            AppItem {
                name: "Firefox".into(),
                monitor: 0,
                workspace: "main".into(),
                pinned: true,
            },
            AppItem {
                name: "Neovim".into(),
                monitor: 1,
                workspace: "dev".into(),
                pinned: false,
            },
        ];
        let list = AppsList {
            items: &items,
            selected: 0,
            scroll_offset: 0,
        };
        let mut buf = Buffer::empty(area(40, 5));
        Widget::render(list, area(40, 5), &mut buf);
    }

    // -- ContextBar --

    #[test]
    fn context_bar_render_does_not_panic() {
        let items = vec![
            ContextItem {
                key: 'q',
                label: "Quit".into(),
                enabled: true,
            },
            ContextItem {
                key: 'm',
                label: "Move".into(),
                enabled: false,
            },
        ];
        let bar = ContextBar { items: &items };
        let mut buf = Buffer::empty(area(60, 1));
        Widget::render(bar, area(60, 1), &mut buf);
    }

    // -- NotificationBar --

    fn buf_content(buf: &Buffer, area: Rect) -> String {
        let mut s = String::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    s.push_str(cell.symbol());
                }
            }
        }
        s.trim_end().to_string()
    }

    #[test]
    fn notification_bar_info_renders() {
        let bar = NotificationBar {
            message: Some("hello"),
            kind: NotificationKind::Info,
        };
        let mut buf = Buffer::empty(area(40, 1));
        Widget::render(bar, area(40, 1), &mut buf);
        assert_eq!(buf_content(&buf, area(40, 1)), "hello");
    }

    #[test]
    fn notification_bar_success_renders() {
        let bar = NotificationBar {
            message: Some("done"),
            kind: NotificationKind::Success,
        };
        let mut buf = Buffer::empty(area(40, 1));
        Widget::render(bar, area(40, 1), &mut buf);
        assert_eq!(buf_content(&buf, area(40, 1)), "done");
    }

    #[test]
    fn notification_bar_error_renders() {
        let bar = NotificationBar {
            message: Some("fail"),
            kind: NotificationKind::Error,
        };
        let mut buf = Buffer::empty(area(40, 1));
        Widget::render(bar, area(40, 1), &mut buf);
        assert_eq!(buf_content(&buf, area(40, 1)), "fail");
    }

    #[test]
    fn notification_bar_no_message_renders_empty() {
        let bar = NotificationBar {
            message: None,
            kind: NotificationKind::TrackChange,
        };
        let mut buf = Buffer::empty(area(40, 1));
        Widget::render(bar, area(40, 1), &mut buf);
        assert_eq!(buf_content(&buf, area(40, 1)), "");
    }

    // -- MonitorDock --

    #[test]
    fn monitor_dock_render_does_not_panic() {
        let monitors = vec![
            MonitorInfo {
                id: 0,
                name: "DP-1".into(),
                focused: true,
            },
            MonitorInfo {
                id: 1,
                name: "HDMI-1".into(),
                focused: false,
            },
        ];
        let dock = MonitorDock {
            monitors: &monitors,
            focused_monitor: 0,
        };
        let mut buf = Buffer::empty(area(30, 1));
        Widget::render(dock, area(30, 1), &mut buf);
    }

    // -- NowBarWidget --

    #[test]
    fn now_bar_widget_render_does_not_panic() {
        let tp = TimeProvider::new();
        let np = NowPlaying::new();
        let widget = NowBarWidget {
            time: &tp,
            track: &np,
        };
        let mut buf = Buffer::empty(area(40, 1));
        Widget::render(widget, area(40, 1), &mut buf);
    }

    #[test]
    fn now_bar_widget_with_track_renders() {
        let tp = TimeProvider::new();
        let mut np = NowPlaying::new();
        np.update(Some(("TestSong", "TestArtist")));
        let widget = NowBarWidget {
            time: &tp,
            track: &np,
        };
        let mut buf = Buffer::empty(area(60, 1));
        Widget::render(widget, area(60, 1), &mut buf);
    }
}
