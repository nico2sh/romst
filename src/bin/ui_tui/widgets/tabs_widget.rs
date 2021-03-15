use crossterm::event::KeyEvent;
use tui::{Frame, backend::Backend, layout::Rect, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, Borders, Tabs}};

use crate::ui_tui::{widgets::{list_db_widget::SelectDBWidget, ViewManager, ViewMessage, error_widget::ErrorWidget, home_widget::HomeWidget}};

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

pub struct TabsMenu<'a, T: Backend> {
    active_menu_item: MenuItem,
    view_manager: ViewManager<'a, T>
}

impl <T: Backend> TabsMenu<'static, T> {
    pub fn new() -> Self {
        let active_menu_item = MenuItem::Home;
        let view_manager = ViewManager::new();

        Self {
            active_menu_item,
            view_manager
        }
    }

    pub fn select_menu_item(&mut self, menu_item: MenuItem) {
        let r = match menu_item {
            MenuItem::Home => {
                let home_widget = HomeWidget::new();
                // self.view_manager.active_widget = Box::new(home_widget);
                self.view_manager.get_sender().send(ViewMessage::NewView(Box::new(home_widget)))
            }
            MenuItem::Database => {
                let db_widget = SelectDBWidget::new();
                // self.view_manager.active_widget = Box::new(db_widget);
                self.view_manager.get_sender().send(ViewMessage::NewView(Box::new(db_widget)))
            }
            MenuItem::Roms => {
                let error_widget = ErrorWidget::new("Generic");
                // self.view_manager.active_widget = Box::new(error_widget);
                self.view_manager.get_sender().send(ViewMessage::NewView(Box::new(error_widget)))
            }
        };
        if let Err(e) = r {
            let error_widget = ErrorWidget::from_error(&e);
            self.view_manager.set_active_widget(Box::new(error_widget));
        }
        self.active_menu_item = menu_item;
    }

    pub fn render_in(&mut self, frame: &mut Frame<T>, area: Rect) {
        self.view_manager.update();

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

    pub fn render_active_widget(&mut self, frame: &mut Frame<T>, area: Rect) {
        self.view_manager.render_active_widget(frame, area);
    }

    pub fn process_keys_active_widget(&mut self, event: KeyEvent) {
        self.view_manager.process_keys_active_widget(event);
    }
}