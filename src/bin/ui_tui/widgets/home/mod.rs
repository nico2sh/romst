use tui::{backend::Backend, layout::Alignment, style::{Color, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, Paragraph}};

use super::RomstController;


pub struct HomeController {
}

impl HomeController {
    pub fn new() -> Self { Self {  } }
}

impl <T: Backend> RomstController<T> for HomeController {
    fn render_view(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
        let paragraph = Paragraph::new(vec![
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::raw("Welcome")]),
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::raw("to")]),
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::styled(
                "Romst",
                Style::default().fg(Color::LightBlue),
            )]),
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::raw("First load or import a Database")]),
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
        // Nothing
    }
}