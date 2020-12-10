pub mod sqlite;

use std::{fmt::Display, collections::{HashMap, HashSet}};

use crate::RomsetMode;

use super::models::{file::DataFile, game::Game, set::GameSet};
use anyhow::Result;

#[derive(Debug)]
pub struct RomSearch {
    pub set_results: HashMap<String, HashSet<DataFile>>,
    pub unknowns: Vec<DataFile>
}

impl Display for RomSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.set_results.len() > 0 {
            for game_roms in &self.set_results {
                writeln!(f, "Set: {}", game_roms.0)?;
                let roms = game_roms.1;
                if roms.len() > 0 {
                    writeln!(f, "  Roms:")?;
                    for rom in roms {
                        writeln!(f, "   - {}", rom)?;
                    }
                }
            }
        }

        if self.unknowns.len() > 0 {
            writeln!(f, "  Unkown files:")?;
            for unknown in &self.unknowns {
                writeln!(f, "   - {}", unknown)?;
            }
        }

        Ok(())
    }
}

impl RomSearch {
    pub fn new() -> Self { Self { set_results: HashMap::new(), unknowns: vec![] } }
    pub fn add_file_for_set(&mut self, set_name: String, file: DataFile) {
        self.set_results.entry(set_name).or_insert(HashSet::new()).insert(file);
    }
    pub fn add_file_unknown(&mut self, file: DataFile) {
        self.unknowns.push(file);
    }
}

pub struct SetResult {
    pub set_name: String,
    pub files: Vec<DataFile>
}

pub trait DataReader {
    fn get_game(&self, game_name: &String) -> Option<Game>;
    fn get_romset_roms(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<Vec<DataFile>>;
    fn get_game_set(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<GameSet> {
        let game = self.get_game(game_name).unwrap();
        let roms = self.get_romset_roms(game_name, rom_mode)?;

        let game_set = GameSet::new(game, roms, vec![], vec![]);
        Ok(game_set)
    }
    /// Finds where this rom is included, in other games. Returns the games and the name used for that rom
    fn find_rom_usage(&self, game_name: &String, rom_name: &String, rom_mode: &RomsetMode) -> Result<RomSearch>;
    /// Gets all romsets that include roms in the searched game
    /// This is useful to know what new (incomplete though) sets can be generated from the current one
    fn get_romset_shared_roms(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<RomSearch>;

    fn get_romsets_from_roms(&self, roms: Vec<DataFile>, rom_mode: &RomsetMode) -> Result<RomSearch>;
}