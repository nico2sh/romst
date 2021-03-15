use tui::{backend::Backend, layout::Alignment, style::{Color, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, Paragraph}};

use super::{RomstWidget, WidgetDispatcher};

pub struct HomeWidget {

}

impl HomeWidget {
    pub fn new() -> Self { Self {  } }
}

impl <'a, T: Backend> RomstWidget<'a, T> for HomeWidget where T: 'a {
    fn render_in(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
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

    }

    fn set_sender(&mut self, _sender: WidgetDispatcher<'a, T>) {
    }
}