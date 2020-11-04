use crate::data::models::{file::DataFile, game::Game};
use anyhow::Result;

use super::DataWriter;

pub struct SysOutWriter {

}

impl SysOutWriter {
    pub fn new() -> Self { Self {  } }
}


impl DataWriter for SysOutWriter {
    fn init(&self) -> Result<()> {
        println!("Initializing...");
        Ok(())
    }

    fn on_new_game(&self, game: &Game) -> Result<()> {
        println!("{}", game.to_string());
        Ok(())
    }

    fn on_new_roms(&mut self, game: &Game, roms: &[DataFile]) -> Result<()> {
        for rom in roms {
            println!(" - [{}] {}: {}", game.name, "File", rom);
        };
        Ok(())
    }
}
