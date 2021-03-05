use tui::{Frame, backend::Backend, layout::{Alignment, Rect}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}};

pub struct Footer {
}

impl Footer {
    pub fn new() -> Self {
        Self {

        }
    }

    pub fn render_in<T: Backend>(&self, frame: &mut Frame<T>, area: Rect) {
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
}