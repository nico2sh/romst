pub mod sqlite;

use std::{collections::{HashMap, HashSet}, fmt::Display, ops::Deref, rc::Rc};

use crate::{RomsetMode, err, error::RomstError, filesystem::FileChecks};
use super::models::{file::DataFile, game::Game, set::GameSet};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use console::Style;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DbDataEntry<T> {
    pub id: u32,
    pub file: T,
}

impl <T> DbDataEntry<T> {
    pub fn new(id: u32, file: T) -> Self { Self { id, file } }
}


impl <T> Display for DbDataEntry<T> where T: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.id, self.file)
    }
}

#[derive(Debug)]
pub struct SetDependencies {
    set_name: String,
    pub dependencies: Vec<String>
}

impl SetDependencies {
    pub fn new<S>(set_name: S) -> Self where S: Into<String> {
        Self {
            set_name: set_name.into(),
            dependencies: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RomSearch {
    searched_roms: HashSet<Rc<DbDataEntry<DataFile>>>,
    pub set_results: HashMap<String, SetContent>,
    pub unknowns: Vec<DataFile>
}

impl RomSearch {
    pub fn new() -> Self {
        Self { searched_roms: HashSet::new(), set_results: HashMap::new(), unknowns: vec![] }
    }
    pub fn add_file_for_set(&mut self, set_name: String, file: DbDataEntry<DataFile>) {
        let set_results = &mut self.set_results;

        let f = Rc::new(file);
        set_results.entry(set_name).or_insert_with(SetContent::new).roms_included.insert(Rc::clone(&f));
        self.searched_roms.insert(f);
    }

    pub fn add_file_unknown(&mut self, file: DataFile) {
        self.unknowns.push(file);
    }

    pub fn get_roms_available_for_set(&self, set: &str) -> Vec<DbDataEntry<DataFile>> {
        if let Some(set_result) = self.set_results.get(set){
            set_result.roms_included.iter().map(|data_file| {
                data_file.deref().to_owned()
            }).collect::<Vec<_>>()
        } else {
            vec![]
        }
    }

    pub fn get_roms_to_spare_for_set(&self, set: &str) -> Vec<DataFile> {
        if let Some(set_result) = self.set_results.get(set) {
            self.searched_roms.iter().filter_map(|rom| {
                if set_result.roms_included.contains(rom) {
                    None
                } else {
                    Some(rom.file.clone())
                }
            }).collect::<Vec<_>>()
        } else {
            vec![]
        }
    }

    // returns the searched roms
    pub fn get_searched_roms(&self) -> Vec<DataFile> {
        let a = self.searched_roms.iter().map(|item| {
            item.file.to_owned()
        }).collect::<Vec<_>>();
        a
    }
}

impl Default for RomSearch {
    fn default() -> Self {
        RomSearch::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetContent {
    roms_included: HashSet<Rc<DbDataEntry<DataFile>>>
}

impl SetContent {
    fn new() -> Self { Self { roms_included: HashSet::new() } }

    pub fn get_roms_included(&self) -> Vec<&DbDataEntry<DataFile>> {
        let a = self.roms_included.iter().map(|data_file| {
            data_file.deref()
        }).collect::<Vec<_>>();
        a
    }
}

impl  Display for RomSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.set_results.is_empty() {
            for game_roms in &self.set_results {
                writeln!(f, "Set: {}", Style::new().green().bold().apply_to(game_roms.0))?;
                let set_content = game_roms.1;
                if !set_content.roms_included.is_empty() {
                    writeln!(f, "  {}:", Style::new().cyan().apply_to("Roms"))?;
                    for rom in &set_content.roms_included {
                        writeln!(f, "   - {}", rom)?;
                    }
                }
            }
        }

        if !self.unknowns.is_empty() {
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
            use_checks &= !FileChecks::SHA1;
        }
        if self.md5 == 0 {
            use_checks &= !FileChecks::MD5;
        }
        if self.crc == 0 {
            use_checks &= !FileChecks::CRC;
        }

        use_checks
    }
}

pub trait DataReader {
    fn get_game_list(&self, rom_mode: RomsetMode) -> Result<Vec<(String, String)>>;
    fn get_game<S>(&self, game_name: S) -> Option<Game> where S: AsRef<str> + rusqlite::ToSql;
    /// Returns all the roms for a specific romset
    fn get_romset_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<Vec<DbDataEntry<DataFile>>> where S: AsRef<str> + rusqlite::ToSql;
    fn get_game_set<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<GameSet> where S: AsRef<str> + rusqlite::ToSql {
        match self.get_game(&game_name) {
            Some(game) => {
                let roms = self.get_romset_roms(game_name.as_ref(), rom_mode)?.into_iter().map(|db_rom| {
                    db_rom.file
                }).collect();
                let device_refs = self.get_devices_for_game(game_name.as_ref())?;
                let game_set = GameSet::new(game, roms, vec![], vec![], device_refs.dependencies);
                Ok(game_set)
            }
            None => err!(RomstError::GenericError{ message: format!("Game {} not found", game_name.as_ref()) }),
        }
    }
    fn get_set_info<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<GameSet> where S: AsRef<str> {
        let roms = self.get_romset_roms(game_name.as_ref(), rom_mode)?.into_iter().map(|db_rom| {
            db_rom.file
        }).collect();
        let device_refs = self.get_devices_for_game(game_name.as_ref())?;
        match self.get_game(game_name.as_ref()) {
            Some(game) => {
                return Ok(GameSet::new(game, roms, vec![], vec![], device_refs.dependencies));
            }
            None => err!(RomstError::GenericError{ message: format!("Game {} not found", game_name.as_ref()) })
        }
    }
    /// Finds where this rom is included, in other games. Returns the games and the name used for that rom
    fn get_rom_usage<S>(&self, game_name: S, rom_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql;
    /// Gets all romsets that include roms in the searched game
    /// This is useful to know what new (incomplete though) sets can be generated from the current one
    fn get_romset_shared_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql;

    /// Finds all romsets associated with the roms sent
    fn get_romsets_from_roms(&self, roms: Vec<DataFile>, rom_mode: RomsetMode) -> Result<RomSearch>;

    fn get_romset_dependencies<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<SetDependencies> where S: AsRef<str> {
        let mut result = self.get_devices_for_game(game_name.as_ref())?;

        // If we are in split mode, we add the parent as a dependency
        match rom_mode {
            RomsetMode::Split => {
                if let Some(game) = self.get_game(game_name.as_ref()) {
                    if let Some(clone_of) = game.clone_of {
                        result.dependencies.push(clone_of);
                    }
                }
            }
            _ => {}
        }

        Ok(result)
    }

    fn get_devices_for_game<S>(&self, game_name: S) -> Result<SetDependencies> where S: AsRef<str> + rusqlite::ToSql;

    fn get_file_checks(&self) -> Result<FileCheckSearch>;
}

#[cfg(test)]
mod tests {
    use super::{DbDataEntry, FileCheckSearch, RomSearch};
    use crate::{data::models::file::{DataFile, DataFileInfo, FileType}, filesystem::FileChecks};

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

    #[test]
    fn should_correctly_add_rom_for_set() {
        let mut rom1 = DataFile::new("rom1", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("8bb3a81b9fa2de5163f0ffc634a998c455bcca25".to_string());
        let mut rom2 = DataFile::new("rom2", DataFileInfo::new(FileType::Rom));
        rom2.info.sha1 = Some("802e076afc412be12db3cb8c79523f65d612a6cf".to_string());
        rom2.info.crc = Some("dc20b010".to_string());
        let mut rom3 = DataFile::new("rom2", DataFileInfo::new(FileType::Rom));
        rom3.info.sha1 = Some("bfa277689790f835d8a43be4beee0581e1096bcc".to_string());
        rom3.info.crc = Some("fbe0d501".to_string());

        let mut rom_search = RomSearch::new();
        rom_search.add_file_for_set("set1".to_string(), DbDataEntry::new(1, rom1));
        rom_search.add_file_for_set("set2".to_string(), DbDataEntry::new(2, rom2));
        rom_search.add_file_for_set("set2".to_string(), DbDataEntry::new(3, rom3));

        let available_1 = rom_search.get_roms_available_for_set(&"set1".to_string());
        let spare_1 = rom_search.get_roms_to_spare_for_set(&"set1".to_string());
        let available_2 = rom_search.get_roms_available_for_set(&"set2".to_string());
        let spare_2 = rom_search.get_roms_to_spare_for_set(&"set2".to_string());

        assert_eq!(1, available_1.len());
        assert_eq!(2, spare_1.len());
        assert_eq!(None, available_1[0].file.info.crc);
        assert!(spare_1.iter().find(|f| { if let Some(crc) = &f.info.crc { crc.eq(&"dc20b010".to_string()) } else { false } }).is_some());
        assert!(spare_1.iter().find(|f| { if let Some(crc) = &f.info.crc { crc.eq(&"fbe0d501".to_string()) } else { false } }).is_some());
        
        assert_eq!(2, available_2.len());
        assert_eq!(1, spare_2.len());
        assert_eq!(None, spare_2[0].info.crc);
        assert!(available_2.iter().find(|f| { if let Some(crc) = &f.file.info.crc { crc.eq(&"dc20b010".to_string()) } else { false } }).is_some());
        assert!(available_2.iter().find(|f| { if let Some(crc) = &f.file.info.crc { crc.eq(&"fbe0d501".to_string()) } else { false } }).is_some());
    }
}