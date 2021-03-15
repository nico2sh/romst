
use std::fmt::Display;

use tui::{backend::Backend, layout::Alignment, style::{Color, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, Paragraph}};

use super::{RomstWidget, WidgetDispatcher};

pub struct ErrorWidget {
    error_message: String
}

impl ErrorWidget {
    pub fn new<S>(error: S) -> Self where S: Into<String> { Self { error_message: error.into() } } 
    pub fn from_error<T>(error: &T) -> Self where T: Display { Self { error_message: format!("{}", error) } } 
}

impl <'a, T: Backend> RomstWidget<'a, T> for ErrorWidget where T: 'a {
    fn render_in(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
        let paragraph = Paragraph::new(vec![
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::raw("Error Loading Screen")]),
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::styled(
                format!("{}", self.error_message),
                Style::default().fg(Color::Red),
            )]),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Home")
                .border_type(BorderType::Plain),
        );

        frame.render_widget(paragraph, area);
    }

    fn process_key(&mut self, _event: crossterm::event::KeyEvent) {

    }

    fn set_sender(&mut self, _sender: WidgetDispatcher<'a, T>) {
        
    }
}