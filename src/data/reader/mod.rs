pub mod sqlite;

use crate::RomsetMode;

use super::models::{file::DataFile, game::Game};
use anyhow::Result;

pub trait DataReader {
    fn get_game(&self, game_name: &String) -> Result<Game>;
    fn get_gameset_roms(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<Vec<DataFile>>;
}