use romst::{RomsetMode, Romst};
use tui::{backend::Backend, widgets::ListState};

use super::RomstWidget;
use anyhow::Result;

struct SetEntry {
    name: String,
    desc: String
}

pub struct DBList {
    db: String,
    set_list: Vec<SetEntry>,
    selected: ListState,
    rom_mode: RomsetMode,
}

impl DBList {
    pub fn new(db: String) -> Result<Self> {
        let rom_mode = RomsetMode::default();
        let set_list = Romst::get_games(&db, rom_mode)?
            .into_iter()
            .map(|item| {
                SetEntry{ name: item.0, desc: item.1 }
            }).collect::<Vec<_>>();
        let selected = ListState::default();

        Ok(Self { db, set_list, selected, rom_mode })
    }
}

impl <T: Backend> RomstWidget<T> for DBList {
    fn render_in(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
        todo!()
    }

    fn process_key(&mut self, event: crossterm::event::KeyEvent) {
        todo!()
    }
}