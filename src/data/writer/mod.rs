pub mod sqlite;
pub mod sysout;
pub mod memory;

use anyhow::Result;

use super::models::{game::Game, file::*};

pub trait DataWriter {
    fn init(&self) -> Result<()>;
    fn on_new_game(&mut self, game: Game) -> Result<()>;
    fn on_new_roms(&mut self, game: Game, roms: Vec<DataFile>) -> Result<()>;
    fn finish(&mut self) -> Result<()>;
}