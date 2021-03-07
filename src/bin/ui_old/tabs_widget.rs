use tui::{Frame, backend::Backend, layout::Rect, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, Borders, Tabs}};

use super::{RomstWidget, select_db_widget::SelectDBWidget, home_widget::HomeWidget};

#[derive(Copy, Clone, Debug)]
pub enum MenuItem {
    Home,
    Database,
    Roms,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Database => 1,
            MenuItem::Roms => 2,
        }
    }
}

pub struct TabsMenu<T: Backend> {
    active_menu_item: MenuItem,
    pub active_widget: Box<dyn RomstWidget<T>>
}

impl <T: Backend> TabsMenu<T> {
    pub fn new() -> Self {
        let active_menu_item = MenuItem::Home;
        let home_widget = HomeWidget::new();

        Self {
            active_menu_item,
            active_widget: Box::new(home_widget)
        }
    }

    pub fn select_menu_item(&mut self, menu_item: MenuItem) {
        match menu_item {
            MenuItem::Home => {
                let home_widget = HomeWidget::new();
                self.active_widget = Box::new(home_widget);
            }
            MenuItem::Database => {
                let db_widget = SelectDBWidget::new();
                self.active_widget = Box::new(db_widget);
            }
            MenuItem::Roms => {}
        };
        self.active_menu_item = menu_item;
    }

    pub fn render_in(&self, frame: &mut Frame<T>, area: Rect) {
        let menu_titles = vec!["Home", "Db", "Roms", "Quit"];
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