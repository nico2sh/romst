use crossterm::event::KeyEvent;
use tui::{Frame, backend::Backend, layout::Rect, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, Borders, Tabs}};

use crate::ui_tui::widgets::ControllerManager;

use super::{ControllerMessage, MenuItem, error::ErrorController};


pub struct TabsMenu {
    pub active_menu_item: MenuItem,
    // controller_manager: ControllerManager<T>
}

impl TabsMenu {
    pub fn new() -> Self {
        let active_menu_item = MenuItem::Home;

        Self {
            active_menu_item,
            // controller_manager
        }
    }

    pub fn render_in<T: Backend>(&mut self, frame: &mut Frame<T>, area: Rect) {
        let menu_titles = vec!["Home", "Db", "GameSets", "Quit"];
        let menu = menu_titles
            .iter()
            .map(|t| {
                let (first, rest) = t.split_at(1);
                Spans::from(vec![
                    Span::styled(
                        first,
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                    Span::styled(rest, Style::default().fg(Color::White)),
                ])
            })
            .collect();
        let tabs = Tabs::new(menu)
            .select(self.active_menu_item.into())
            .block(Block::default().title("Menu").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(Span::raw("|"));

        frame.render_widget(tabs, area);
    }
}