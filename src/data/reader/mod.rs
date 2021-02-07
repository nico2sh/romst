pub mod sqlite;

use std::{collections::{self, HashMap, HashSet}, fmt::Display};

use crate::{RomsetMode, err, error::RomstError, filesystem::FileChecks};
use super::models::{file::DataFile, game::Game, set::GameSet};
use anyhow::Result;
use collections::hash_map;
use console::Style;
use hash_map::Entry;
use log::warn;

#[derive(Debug)]
pub struct RomSearch<'a> {
    searched_roms: HashSet<DataFile>,
    pub set_results: HashMap<String, SetContent<'a>>,
    pub unknowns: Vec<DataFile>
}

impl <'a> RomSearch<'a> {
    pub fn new<I>(searched_roms: I) -> Self where I: IntoIterator<Item = DataFile> {
        Self { searched_roms: searched_roms.into_iter().collect(), set_results: HashMap::new(), unknowns: vec![] }
    }
    /*pub fn new<I>() -> Self {
        Self { searched_roms: HashSet::new(), set_results: HashMap::new(), unknowns: vec![] }
    }*/
    pub fn add_file_for_set(&mut self, set_name: String, file: &DataFile) {
        match self.set_results.entry(set_name) {
            Entry::Occupied(mut entry) => {
                let set_content = entry.get_mut();
                if let Some(found) = set_content.roms_to_spare.take(file) {
                    set_content.roms_included.insert(found);
                } else {
                    warn!("File {} not in searching roms", file);
                };
            }
            Entry::Vacant(vacant_entry) => {
                let mut set_content = SetContent::new();
                for item in &self.searched_roms {
                    if item.eq(file) {
                        //set_content.roms_included.insert(item);
                    } else {
                        //set_content.roms_to_spare.insert(item);
                    }
                }
                /*self.searched_roms.iter().for_each(|item| {
                    if item.eq(file) {
                        set_content.roms_included.insert(item);
                    } else {
                        set_content.roms_to_spare.insert(item);
                    }
                });*/
                vacant_entry.insert(set_content);
            }
        }
    }

    fn add_rom_included() {

    }

    pub fn add_file_unknown(&mut self, file: DataFile) {
        self.unknowns.push(file);
    }
}

#[derive(Debug)]
pub struct SetContent<'a> {
    roms_included: HashSet<&'a DataFile>,
    roms_to_spare: HashSet<&'a DataFile>
}

impl <'a> SetContent<'a> {
    fn new() -> Self { Self { roms_included: HashSet::new(), roms_to_spare: HashSet::new() } }
    pub fn get_roms_included(&self) -> Vec<&DataFile> {
        let a = self.roms_included.iter().map(|data_file| {
            data_file.clone()
        }).collect::<Vec<_>>();
        a
    }
}

impl <'a> Display for RomSearch<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.set_results.len() > 0 {
            for game_roms in &self.set_results {
                writeln!(f, "Set: {}", Style::new().green().bold().apply_to(game_roms.0))?;
                let set_content = game_roms.1;
                if set_content.roms_included.len() > 0 {
                    writeln!(f, "  {}:", Style::new().cyan().apply_to("Roms"))?;
                    for rom in &set_content.roms_included {
                        writeln!(f, "   - {}", rom)?;
                    }
                }
            }
        }

        if self.unknowns.len() > 0 {
            writeln!(f, "  {}:", Style::new().red().apply_to("Unkown files"))?;
            for unknown in &self.unknowns {
                writeln!(f, "   - {}", unknown)?;
            }
        }

        Ok(())
    }
}

pub struct FileCheckSearch {
    pub sha1: u32,
    pub md5: u32,
    pub crc: u32
}

impl FileCheckSearch {
    pub fn get_file_checks(&self) -> FileChecks {
        let mut use_checks = FileChecks::ALL;
        if self.sha1 == 0 {
            use_checks = use_checks & !FileChecks::SHA1;
        }
        if self.md5 == 0 {
            use_checks = use_checks & !FileChecks::MD5;
        }
        if self.crc == 0 {
            use_checks = use_checks & !FileChecks::CRC;
        }

        use_checks
    }
}

pub trait DataReader {
    fn get_game<S>(&self, game_name: S) -> Option<Game> where S: AsRef<str> + rusqlite::ToSql;
    fn get_romset_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<Vec<DataFile>> where S: AsRef<str> + rusqlite::ToSql;
    fn get_game_set<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<GameSet> where S: AsRef<str> + rusqlite::ToSql {
        match self.get_game(&game_name) {
            Some(game) => {
                let roms = self.get_romset_roms(game_name, rom_mode)?;
                let game_set = GameSet::new(game, roms, vec![], vec![]);
                Ok(game_set)
            }
            None => err!(RomstError::GenericError{ message: format!("Game {} not found", game_name.as_ref()) }),
        }
    }
    /// Finds where this rom is included, in other games. Returns the games and the name used for that rom
    fn find_rom_usage<S>(&self, game_name: S, rom_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql;
    /// Gets all romsets that include roms in the searched game
    /// This is useful to know what new (incomplete though) sets can be generated from the current one
    fn get_romset_shared_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql;

    /// Finds all romsets associated with the roms sent
    fn get_romsets_from_roms(&self, roms: Vec<DataFile>, rom_mode: RomsetMode) -> Result<RomSearch>;

    fn get_devices_for_game<S>(&self, game_name: S) -> Result<Vec<String>> where S: AsRef<str> + rusqlite::ToSql;

    fn get_file_checks(&self) -> Result<FileCheckSearch>;
}

#[cfg(test)]
mod tests {
    use super::FileCheckSearch;
    use crate::filesystem::FileChecks;

    #[test]
    fn should_check_with_all() {
        let file_check_search = FileCheckSearch {
            sha1: 1,
            md5: 1,
            crc: 1
        };
        let file_checks = file_check_search.get_file_checks();

        assert!(file_checks.contains(FileChecks::SHA1));
        assert!(file_checks.contains(FileChecks::MD5));
        assert!(file_checks.contains(FileChecks::CRC));
    }

    #[test]
    fn should_check_without_md5() {
        let file_check_search = FileCheckSearch {
            sha1: 1,
            md5: 0,
            crc: 1
        };
        let file_checks = file_check_search.get_file_checks();

        assert!(file_checks.contains(FileChecks::SHA1));
        assert!(!file_checks.contains(FileChecks::MD5));
        assert!(file_checks.contains(FileChecks::CRC));
    }

    #[test]
    fn should_check_without_sha1() {
        let file_check_search = FileCheckSearch {
            sha1: 0,
            md5: 1,
            crc: 1
        };
        let file_checks = file_check_search.get_file_checks();

        assert!(!file_checks.contains(FileChecks::SHA1));
        assert!(file_checks.contains(FileChecks::MD5));
        assert!(file_checks.contains(FileChecks::CRC));
    }

    #[test]
    fn should_check_without_sha1_or_md5() {
        let file_check_search = FileCheckSearch {
            sha1: 0,
            md5: 0,
            crc: 1
        };
        let file_checks = file_check_search.get_file_checks();

        assert!(!file_checks.contains(FileChecks::SHA1));
        assert!(!file_checks.contains(FileChecks::MD5));
        assert!(file_checks.contains(FileChecks::CRC));
    }

    #[test]
    fn should_check_without_crc() {
        let file_check_search = FileCheckSearch {
            sha1: 1,
            md5: 1,
            crc: 0
        };
        let file_checks = file_check_search.get_file_checks();

        assert!(file_checks.contains(FileChecks::SHA1));
        assert!(file_checks.contains(FileChecks::MD5));
        assert!(!file_checks.contains(FileChecks::CRC));
    }
}