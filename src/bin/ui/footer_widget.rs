use tui::{Frame, backend::Backend, layout::{Alignment, Rect}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph, Widget}};
use anyhow::Result;

use super::RomstWidget;

pub struct Footer {
}

impl Footer {
    pub fn new() -> Self {
        Self {

        }
    }
}

impl <T: Backend> RomstWidget<T> for Footer {
    fn render_in(&mut self, frame: &mut Frame<T>, area: Rect) {
        let paragraph = Paragraph::new("Romst, a Rom checker written in Rust (press 'q' to quit)")
            .style(Style::default().fg(Color::LightCyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title("About")
                    .border_type(BorderType::Plain),
            );
        frame.render_widget(paragraph, area);
    }

    fn process_key(&mut self, event: crossterm::event::KeyEvent) {

    }
}