pub mod sqlite;

use crate::RomsetMode;

use super::models::{file::DataFile, game::Game, report::Report};
use anyhow::Result;

pub trait DataReader {
    fn get_game(&self, game_name: &String) -> Result<Game>;
    fn get_romset_roms(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<Vec<DataFile>>;
    /// Finds where this rom is included, in other games. Returns the games and the name used for that rom
    fn find_rom_usage(&self, game_name:&String, rom_name: &String) -> Result<Vec<Report>>;
    /// Gets all romsets that include roms in the searched game
    /// This is useful to know what new (incomplete though) sets can be generated from the current one
    fn get_romset_shared_roms(&self, game_name: &String) -> Result<Vec<Report>>;
}