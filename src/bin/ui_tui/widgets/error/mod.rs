use tui::{backend::Backend, layout::Alignment, style::{Color, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, Paragraph}};

use super::RomstController;


pub struct ErrorController {
    error_message: String
}

impl ErrorController {
    pub fn new<S>(error_message: S) -> Self where S: Into<String> { Self { error_message: error_message.into() } }
}

impl <T: Backend> RomstController<T> for ErrorController {
    fn render_view(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
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
        // meh
    }
}