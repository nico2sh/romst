use romst::data::reader::sqlite::DBReport;
use tui::{backend::Backend, layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Span, Spans, Text}, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap}};

use crate::ui_tui::widgets::RomstView;

pub enum DBDetails {
    None,
    Info(DBReport),
    Error(String)
}

#[derive(Clone)]
pub struct SelectDBFileEntry {
    file_name: String,
    pub path: String
}

impl SelectDBFileEntry {
    fn new(file_name: String, path: String) -> Self { Self { file_name, path } }
}

impl <'a> Into<Text<'a>> for SelectDBFileEntry {
    fn into(self) -> Text<'a> {
        Spans::from(Span::styled(self.file_name.clone(), Style::default().add_modifier(Modifier::BOLD))).into()
    }
}

pub struct DBFileView {
    pub db_list: Vec<SelectDBFileEntry>,
    pub details: DBDetails,
    pub selected: ListState,
}

impl DBFileView {
    pub fn new(db_file_list: Vec<(String, String)>) -> Self {
        let db_list = db_file_list.into_iter().map(|file_path| {
            SelectDBFileEntry::new(file_path.0.clone(), file_path.1.clone())
        }).collect::<Vec<_>>();
        let details = DBDetails::None;
        let selected = ListState::default();

        Self { db_list, details, selected }
    }

    pub fn get_db_file_list<'a>(&self) -> List<'a> {
        let files = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Db Files")
            .border_type(BorderType::Plain);
        
        let items = self.db_list.iter().map(|entry| ListItem::new(entry.clone())).collect::<Vec<_>>();
        let db_list = List::new(items).block(files).highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

        db_list
    }

    fn get_db_detail_widget<'a>(&self) -> Paragraph<'a> {
        match &self.details {
            DBDetails::None => {
                Paragraph::new("")
            }
            DBDetails::Info(db_info) => {
                get_db_details_view(db_info)
            }
            DBDetails::Error(e) => {
                get_error_widget(e)
            }
        }
    }

}

impl <T: Backend> RomstView<T> for DBFileView {
    fn render_in(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
            )
            .split(area);

        frame.render_stateful_widget(self.get_db_file_list(), chunks[0], &mut self.selected);
        frame.render_widget(self.get_db_detail_widget(), chunks[1]);
    }
}

fn get_db_details_view<'a>(db_info: &DBReport) -> Paragraph<'a> {
    let mut text = vec![
        Spans::from(vec![
            Span::styled(format!("Name: {}", db_info.dat_info.name), Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Spans::from(vec![
            Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(db_info.dat_info.description.to_owned(), Style::default()),
        ]),
        Spans::from(vec![
            Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(db_info.dat_info.version.to_owned(), Style::default()),
        ]),
    ];

    for extra in &db_info.dat_info.extra_data {
        text.push(
            Spans::from(vec![
                Span::styled(format!("{}: ", extra.0), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(extra.1.to_owned()),
            ])
        )
    }

    text.push(Spans::from(Span::raw("")));

    text.extend(vec![
        Spans::from(Span::raw(format!("Games: {}", db_info.games.to_string()))),
        Spans::from(Span::raw(format!("Roms: {}", db_info.roms.to_string()))),
        Spans::from(Span::raw(format!("Roms in Games: {}", db_info.roms_in_games.to_string()))),
        Spans::from(Span::raw(format!("Samples: {}", db_info.samples.to_string()))),
        Spans::from(Span::raw(format!("Device Refs: {}", db_info.device_refs.to_string()))),
    ]);

    let paragraph = Paragraph::new(text)
    .block(
        Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Detail")
        .border_type(BorderType::Rounded),
    )
    .wrap(Wrap { trim: true });

    return paragraph;
}

fn get_error_widget<'a, S>(error: S) -> Paragraph<'a> where S: AsRef<str> {
    let p = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Error")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("loading Details")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            format!("{}", error.as_ref()),
            Style::default().fg(Color::Red),
        )]),
        Spans::from(vec![Span::raw("")]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Home")
            .border_type(BorderType::Plain),
    );

    return p;
}