use std::path::Path;
use std::fs;
use anyhow::{Error, Result, anyhow};
use crossterm::event::KeyCode;
use romst::Romst;
use romst::data::reader::sqlite::DBReport;
use tui::{Frame, backend::Backend, layout::{Alignment, Constraint, Layout, Rect}, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Widget}};

use super::RomstWidget;

const BASE_PATH: &str = "db";

enum OptionSelected {
    Import,
    DbInfo(DBReport),
    Err(Error)
}

pub struct DBWidget {
    db_list: Vec<String>,
    selected: ListState,
    option_selected: OptionSelected
}

impl DBWidget {
    pub fn new() -> Self {
        let db_list = DBWidget::get_db_list().unwrap_or_else(|_e| vec![]);
        let mut selected = ListState::default();
        selected.select(Some(0));
        Self {
            db_list,
            selected,
            option_selected: OptionSelected::Import
        }
    }

    fn get_file_list<'a>(&self) -> Vec<ListItem<'a>> {
        self.db_list.iter().map(|s| {
            self.get_list_item(s)
        }).collect::<Vec<_>>()
    }

    fn get_db_list() -> Result<Vec<String>> {
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
                        f.file_name().to_str().map(|s| s.to_string() )
                    } else {
                        None
                    }
                }
                Err(_) => None
            }
        }).collect::<Vec<_>>();

        files.insert(0, "[IMPORT DAT FILE]".to_string());

        Ok(files)
    }

    fn get_list_item<'a>(&self, text: &str) -> ListItem<'a> {
        ListItem::new(Spans::from(vec![Span::styled(
            text.to_string(),
            Style::default(),
        )]))
    }

    fn get_db_detail_widget<'a>(db_info: &DBReport) -> Table<'a> {
        let db_detail = Table::new(vec![Row::new(vec![
            Cell::from(Span::raw(db_info.games.to_string())),
            Cell::from(Span::raw(db_info.roms.to_string())),
            Cell::from(Span::raw(db_info.roms_in_games.to_string())),
            Cell::from(Span::raw(db_info.samples.to_string())),
            Cell::from(Span::raw(db_info.device_refs.to_string())),
        ])])
        .header(Row::new(vec![
            Cell::from(Span::styled(
                "File",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Games",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Roms",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Roms in Games",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Samples",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Device Refs",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Detail")
                .border_type(BorderType::Plain),
        )
        .widths(&[
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
        ]);

        return db_detail;
    }

    fn get_import_db_widget<'a>() -> Paragraph<'a> {
        let p = Paragraph::new(vec![
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::raw("Import")]),
            Spans::from(vec![Span::raw("")]),
            Spans::from(vec![Span::raw("a DAT file")]),
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

    fn update_selected(&mut self) {
        if let Some(selected) = self.selected.selected() {
            let option_selected = if selected == 0 {
                OptionSelected::Import
            } else {
                if let Some(db) = self.db_list.get(selected) {
                    match Romst::get_db_info(db) {
                        Ok(info) => {
                            OptionSelected::DbInfo(info)
                        }
                        Err(e) => {
                            OptionSelected::Err(e.into())
                        }
                    }
                } else {
                    OptionSelected::Err(anyhow!("Unknown Error"))
                }
            };
            self.option_selected = option_selected;
        }
    }
}

impl <T: Backend> RomstWidget<T> for DBWidget {
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
                let widget = DBWidget::get_import_db_widget();
                frame.render_widget(widget, chunks[1]);
            }
            OptionSelected::DbInfo(db_info) => {
                let widget = DBWidget::get_db_detail_widget(db_info);
                frame.render_widget(widget, chunks[1]);
            }
            OptionSelected::Err(error) => {
                let widget = DBWidget::get_error_widget(error);
                frame.render_widget(widget, chunks[1]);
            }
        }
    }

    fn process_key(&mut self, event: crossterm::event::KeyEvent) {
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

            },
            _ => {}
        }
    }
}