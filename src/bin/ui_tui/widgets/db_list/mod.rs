use std::{fs, path::Path};

use anyhow::Result;
use crossterm::event::KeyCode;
use romst::{Romst};
use tui::{Frame, backend::Backend};

use self::view::{DBDetails, DBFileView};

use super::{ControllerDispatcher, ControllerMessage, RomstController, RomstView};

mod view;

const BASE_PATH: &str = "db";

pub struct DBListController {
    view: DBFileView,
    sender: ControllerDispatcher
}

impl DBListController {
    pub fn new(sender: ControllerDispatcher) -> Self {
        let view = DBFileView::new(get_db_list().unwrap());
        Self { view, sender }
    }

    fn update_selected(&mut self) {
        match self.view.selected.selected() {
            Some(selected) => {
                self.update_view_selection(selected);
            },
            None => {
                self.view.details = DBDetails::None;
            }
        }
    }

    fn update_view_selection(&mut self, pos: usize) {
        match Romst::get_db_info(&self.view.db_list[pos].path) {
            Ok(info) => {
                self.view.details = DBDetails::Info(info);
            }
            Err(e) => {
                self.view.details = DBDetails::Error(format!("{}", e));
            }
        }
    }
}

impl <T: Backend> RomstController<T> for DBListController {
    fn render_view(&mut self, frame: &mut Frame<T>, area: tui::layout::Rect) {
        self.view.render_in(frame, area);
    }

    fn process_key(&mut self, event: crossterm::event::KeyEvent) {
        match event.code {
            KeyCode::Down => {
                let entries = self.view.db_list.len();
                if let Some(selected) = self.view.selected.selected() {
                    if selected >= entries - 1 {
                        self.view.selected.select(Some(0));
                    } else {
                        self.view.selected.select(Some(selected + 1));
                    }
                } else {
                    if entries > 0 {
                        self.view.selected.select(Some(0));
                    }
                };
                self.update_selected();
            },
            KeyCode::Up => {
                let entries = self.view.db_list.len();
                if let Some(selected) = self.view.selected.selected() {
                    if selected > 0 {
                        self.view.selected.select(Some(selected - 1));
                    } else {
                        self.view.selected.select(Some(entries - 1));
                    }
                } else {
                    if entries > 0 {
                        self.view.selected.select(Some(entries - 1));
                    }
                };
                self.update_selected();
            },
            KeyCode::Enter => {
                if let Some(selected) = self.view.selected.selected() {
                    if let Some(db_entry) = self.view.db_list.get(selected) {
                        self.sender.send(ControllerMessage::DBSelected(db_entry.path.clone())).unwrap();
                    };
                }
            },
            _ => {}
        }
    }
}

fn get_db_list() -> Result<Vec<(String, String)>> {
    let db_path = Path::new(BASE_PATH);

    if db_path.is_file() {
        fs::remove_file(db_path)?;
    };

    if !db_path.exists() {
        fs::create_dir(db_path)?;
    };

    let files = db_path.read_dir()?.into_iter().filter_map(|file| {
        match file {
            Ok(f) => { 
                let path = f.path();
                if path.is_file() {
                    let file_name = f.file_name().to_str().map(|s| s.to_string() );
                    let path_string = path.to_str().map(|s| s.to_string() );

                    if let (Some(l), Some(r)) = (file_name, path_string) {
                        Some((l, r))
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

    Ok(files)
}
