//! Help overlay — toggled by `?`, dismissed by `Esc` / `?`.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::ui::widgets::centered_rect;

struct HelpItem {
    key: &'static str,
    description: &'static str,
}

fn help_items() -> Vec<HelpItem> {
    vec![
        HelpItem {
            key: "1-4",
            description: "Jump to view",
        },
        HelpItem {
            key: "Tab",
            description: "Next view",
        },
        HelpItem {
            key: "Shift+Tab",
            description: "Previous view",
        },
        HelpItem {
            key: "j / k / ↓ / ↑",
            description: "Sidebar: switch view · rows: Projects, Findings",
        },
        HelpItem {
            key: "h / ←",
            description: "Focus sidebar",
        },
        HelpItem {
            key: "l / →",
            description: "Focus main",
        },
        HelpItem {
            key: "g / G",
            description: "First / last row (Projects, Findings)",
        },
        HelpItem {
            key: "r",
            description: "Refresh (Projects) · re-dig (Findings)",
        },
        HelpItem {
            key: "v",
            description: "Projects: Cards → Table → Tree",
        },
        HelpItem {
            key: "Enter",
            description: "Activate sidebar item · dig selected project",
        },
        HelpItem {
            key: "R",
            description: "Raid selected project (Projects): opens the flow",
        },
        HelpItem {
            key: "K",
            description: "Raid flow: keep sources (toggle)",
        },
        HelpItem {
            key: "S",
            description: "Raid flow: skip stash (toggle)",
        },
        HelpItem {
            key: "m",
            description: "Raid flow: mode Atomic ↔ Fail-Fast",
        },
        HelpItem {
            key: "f",
            description: "Findings: filter by min risk",
        },
        HelpItem {
            key: "c",
            description: "Findings: toggle content scan (re-dig)",
        },
        HelpItem {
            key: "?",
            description: "Toggle help",
        },
        HelpItem {
            key: "Esc",
            description: "Close help · focus sidebar · back to Projects",
        },
        HelpItem {
            key: "q",
            description: "Quit",
        },
        HelpItem {
            key: "Ctrl-C",
            description: "Force quit",
        },
    ]
}

/// Render the help dialog centered in `area`.
pub fn render(f: &mut Frame, area: Rect) {
    let popup = centered_rect(65, 85, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BRAND_PRIMARY))
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));

    let header = Line::from(vec![
        Span::styled(
            "  Key",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "    Description",
            Style::default()
                .fg(theme::BRAND_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let mut lines = vec![header, Line::from("")];
    for item in help_items() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:10}", item.key), Style::default().fg(theme::FG)),
            Span::styled(item.description, Style::default().fg(theme::MUTED)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  Press Esc or ? to close",
        Style::default()
            .fg(theme::MUTED)
            .add_modifier(Modifier::ITALIC),
    )]));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().bg(theme::BG).fg(theme::FG)),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_items_non_empty() {
        assert!(!help_items().is_empty());
    }

    #[test]
    fn help_items_have_keys_and_descriptions() {
        for item in help_items() {
            assert!(!item.key.is_empty(), "key must not be empty");
            assert!(
                !item.description.is_empty(),
                "description must not be empty"
            );
        }
    }

    #[test]
    fn help_items_cover_global_keys() {
        let keys: Vec<_> = help_items().iter().map(|i| i.key).collect();
        for expected in [
            "Tab",
            "Shift+Tab",
            "j / k / ↓ / ↑",
            "h / ←",
            "l / →",
            "1-4",
            "?",
            "Esc",
            "q",
        ] {
            assert!(
                keys.contains(&expected),
                "help must document {expected:?}, got {keys:?}"
            );
        }
    }

    #[test]
    fn help_items_cover_dig_keys() {
        let keys: Vec<_> = help_items().iter().map(|i| i.key).collect();
        for expected in ["f", "c", "r", "Enter"] {
            assert!(
                keys.contains(&expected),
                "help must document {expected:?}, got {keys:?}"
            );
        }
    }

    #[test]
    fn help_items_cover_raid_keys() {
        let keys: Vec<_> = help_items().iter().map(|i| i.key).collect();
        for expected in ["R", "K", "S", "m"] {
            assert!(
                keys.contains(&expected),
                "help must document the raid keys {expected:?}, got {keys:?}"
            );
        }
    }

    #[test]
    fn help_items_cover_projects_mode_key() {
        let keys: Vec<_> = help_items().iter().map(|i| i.key).collect();
        assert!(
            keys.contains(&"v"),
            "help must document the projects mode key 'v', got {keys:?}"
        );
    }
}
