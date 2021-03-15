use std::{path::Path};
use std::fs;
use anyhow::{Error, Result, anyhow};
use crossterm::event::{KeyCode, KeyEvent};
use romst::Romst;
use romst::data::reader::sqlite::DBReport;
use tui::{Frame, backend::Backend, layout::{Alignment, Constraint, Layout, Rect}, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap}};

use super::{RomstWidget, ViewMessage, WidgetDispatcher, list_set_widget::SetListWidget};

const BASE_PATH: &str = "db";

enum OptionSelected {
    Import,
    DbInfo(DBReport),
    Err(Error)
}

enum SelectDBEntry {
    Import, 
    File(SelectDBFileEntry)
}

impl SelectDBEntry {
    fn get_entry_title(&self) -> String {
        match self {
            SelectDBEntry::Import => "[IMPORT DAT FILE]".to_string(),
            SelectDBEntry::File(entry) => entry.file_name.clone()
        }
    }
}

struct SelectDBFileEntry {
    file_name: String,
    path: String
}

impl SelectDBFileEntry {
    fn new(file_name: String, path: String) -> Self { Self { file_name, path } }
}

pub struct SelectDBWidget<'a, T: Backend> {
    tx: Option<WidgetDispatcher<'a, T>>,
    db_list: Vec<SelectDBEntry>,
    selected: ListState,
    option_selected: OptionSelected
}

impl <'a, T: Backend> SelectDBWidget<'a, T> {
    pub fn new() -> Self {
        let db_list = get_db_list().unwrap_or_else(|_e| vec![]);
        let mut selected = ListState::default();
        selected.select(Some(0));
        Self {
            tx: None,
            db_list,
            selected,
            option_selected: OptionSelected::Import
        }
    }

    fn get_file_list<'b>(&self) -> Vec<ListItem<'b>> {
        self.db_list.iter().map(|s| {
            self.get_list_item(s.get_entry_title().as_str())
        }).collect::<Vec<_>>()
    }

    fn get_list_item<'b>(&self, text: &str) -> ListItem<'b> {
        ListItem::new(Spans::from(vec![Span::styled(
            text.to_string(),
            Style::default(),
        )]))
    }

    fn update_selected(&mut self) {
        if let Some(selected) = self.selected.selected() {
            let option_selected = if let Some(db_entry) = self.db_list.get(selected) {
                match db_entry {
                    SelectDBEntry::Import => {
                        OptionSelected::Import
                    }
                    SelectDBEntry::File(file_entry) => {
                        match Romst::get_db_info(&file_entry.path) {
                            Ok(info) => {
                                OptionSelected::DbInfo(info)
                            }
                            Err(e) => {
                                OptionSelected::Err(e.into())
                            }
                        }
                    }
                }
            } else {
                OptionSelected::Err(anyhow!("Unknown Error"))
            };

            self.option_selected = option_selected;
        }
    }
}

impl <'a, T: Backend> RomstWidget<'a, T> for SelectDBWidget<'a, T> {
    fn render_in(&mut self, frame: &mut Frame<T>, area: Rect) {
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
            )
            .split(area);
        
        let files = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Db Files")
            .border_type(BorderType::Plain);

        let items = self.get_file_list();

        let list = List::new(items).block(files).highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_stateful_widget(list, chunks[0], &mut self.selected);

        match &self.option_selected {
            OptionSelected::Import => {
                let widget = get_import_db_widget();
                frame.render_widget(widget, chunks[1]);
            }
            OptionSelected::DbInfo(db_info) => {
                let widget = get_db_detail_widget(db_info);
                frame.render_widget(widget, chunks[1]);
            }
            OptionSelected::Err(error) => {
                let widget = get_error_widget(error);
                frame.render_widget(widget, chunks[1]);
            }
        }
    }

    fn process_key(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Down => {
                let entries = self.db_list.len();
                if let Some(selected) = self.selected.selected() {
                    if selected >= entries - 1 {
                        self.selected.select(Some(0));
                    } else {
                        self.selected.select(Some(selected + 1));
                    }
                    self.update_selected();
                };
            },
            KeyCode::Up => {
                let entries = self.db_list.len();
                if let Some(selected) = self.selected.selected() {
                    if selected > 0 {
                        self.selected.select(Some(selected - 1));
                    } else {
                        self.selected.select(Some(entries - 1));
                    }
                    self.update_selected();
                };
            },
            KeyCode::Enter => {
                if let Some(selected) = self.selected.selected() {
                    if let Some(db_entry) = self.db_list.get(selected) {
                        if let SelectDBEntry::File(db_file_entry) = db_entry {
                            let set_list = SetListWidget::new(db_file_entry.path.clone()).unwrap();
                            if let Some(tx) = &self.tx {
                                tx.send(ViewMessage::NewView(Box::new(set_list))).unwrap();
                            };
                        }
                    };
                }
            },
            _ => {}
        }
    }

    fn set_sender(&mut self, sender: WidgetDispatcher<'a, T>) {
        self.tx = Some(sender);
    }
}

fn get_db_list() -> Result<Vec<SelectDBEntry>> {
    let db_path = Path::new(BASE_PATH);

    if db_path.is_file() {
        fs::remove_file(db_path)?;
    };

    if !db_path.exists() {
        fs::create_dir(db_path)?;
    };

    let mut files = db_path.read_dir()?.into_iter().filter_map(|file| {
        match file {
            Ok(f) => { 
                let path = f.path();
                if path.is_file() {
                    let file_name = f.file_name().to_str().map(|s| s.to_string() );
                    let path_string = path.to_str().map(|s| s.to_string() );

                    if let (Some(l), Some(r)) = (file_name, path_string) {
                        Some(SelectDBEntry::File(SelectDBFileEntry::new(l, r)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None
        }
    }).collect::<Vec<_>>();

    files.insert(0, SelectDBEntry::Import);

    Ok(files)
}

fn get_import_db_widget<'a>() -> Paragraph<'a> {
    let p = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Import")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("a DAT file")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("(Work in progress)")]),
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

fn get_db_detail_widget<'a>(db_info: &'a DBReport) -> Paragraph<'a> {
    let mut text = vec![
        Spans::from(vec![
            Span::styled(format!("Name: {}", db_info.dat_info.name), Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Spans::from(vec![
            Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(&db_info.dat_info.description, Style::default()),
        ]),
        Spans::from(vec![
            Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(&db_info.dat_info.version, Style::default()),
        ]),
    ];

    for extra in &db_info.dat_info.extra_data {
        text.push(
            Spans::from(vec![
                Span::styled(format!("{}: ", &extra.0), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&extra.1),
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

fn get_error_widget<'a>(error: &Error) -> Paragraph<'a> {
    let p = Paragraph::new(vec![
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("Error")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::raw("loading Details")]),
        Spans::from(vec![Span::raw("")]),
        Spans::from(vec![Span::styled(
            format!("{}", error),
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
