use crossterm::event::KeyCode;
use romst::{RomsetMode, Romst, data::reader::{DataReader, sqlite::DBReader}};
use tui::backend::Backend;

mod view;

use self::view::{SetDetails, SetListView};

use super::{RomstController, RomstView};


pub struct SetListController {
    db_reader: DBReader,
    rom_mode: RomsetMode,
    view: SetListView,
}

impl SetListController {
    pub fn new<S>(db_file: S) -> Self where S: Into<String> {
        let db_reader = Romst::get_data_reader(db_file.into()).unwrap();
        let rom_mode = RomsetMode::default();
        let sets = db_reader.get_game_list(rom_mode).unwrap();
        let view = SetListView::new(sets);
        Self { db_reader, rom_mode, view }
    }

    fn update_selected(&mut self) {
        if let Some(selected) = self.view.selected.selected() {
            if let Some(set_entry) = self.view.set_list.get(selected) {
                match self.db_reader.get_set_info(&set_entry.name, self.rom_mode) {
                    Ok(set) => {
                        self.view.details = SetDetails::GameSet(set);
                    }
                    Err(e) => {
                        self.view.details = SetDetails::Error(format!("{}", e));
                    }
                }
            };
        };
    }
}

impl <T: Backend> RomstController<T> for SetListController {
    fn render_view(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
        self.view.render_in(frame, area);
    }

    fn process_key(&mut self, event: crossterm::event::KeyEvent) {
        match event.code {
            KeyCode::Down => {
                let entries = self.view.set_list.len();
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
                let entries = self.view.set_list.len();
                if let Some(selected) = self.view.selected.selected() {
                    if selected > 0 {
                        self.view.selected.select(Some(selected - 1));
                    } else {
                        self.view.selected.select(Some(entries - 1));
                    }
                } else {
                    if entries > 0 {
                        self.view.selected.select(Some(0));
                    }
                };
                self.update_selected();
            },
            KeyCode::PageDown => {
                let entries = self.view.set_list.len();
                if let Some(selected) = self.view.selected.selected() {
                    let new_value = std::cmp::min(entries - 1, selected + 10);
                    self.view.selected.select(Some(new_value));
                } else {
                    if entries > 0 {
                        self.view.selected.select(Some(0));
                    }
                };
                self.update_selected();
            },
            KeyCode::PageUp => {
                let entries = self.view.set_list.len();
                if let Some(selected) = self.view.selected.selected() {
                    let new_value = std::cmp::max(0, selected - 10);
                    self.view.selected.select(Some(new_value));
                } else {
                    if entries > 0 {
                        self.view.selected.select(Some(0));
                    }
                };
                self.update_selected();
            }
            _ => {}
        }
    }
}