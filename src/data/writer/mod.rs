pub mod sqlite;
pub mod sysout;

use anyhow::Result;

use super::models::{game::Game, file::*};

pub trait DataWriter {
    fn init(&self) -> Result<()>;
    fn on_new_game(&self, game: &Game) -> Result<()>;
    fn on_new_roms(&mut self, game: &Game, roms: &[DataFile]) -> Result<()>;
}