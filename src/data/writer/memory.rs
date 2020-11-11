use anyhow::Result;

use crate::data::models::{file::DataFile, game::Game};

use super::DataWriter;

pub struct MemoryWriter {
    pub initialized: bool,
}

impl MemoryWriter {
    pub fn new() -> Self {
        MemoryWriter {
            initialized: false,
        }
    }
}

impl DataWriter for MemoryWriter {
    fn init(&self) -> Result<()> {
        Ok(())
    }

    fn on_new_game(&mut self, game: Game) -> Result<()> {
        Ok(())
    }

    fn on_new_roms(&mut self, game: Game, roms: Vec<DataFile>) -> Result<()> {
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}