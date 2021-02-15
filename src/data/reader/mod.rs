pub mod sqlite;

use std::{collections::{HashMap, HashSet}, fmt::Display, ops::Deref, rc::Rc};

use crate::{RomsetMode, err, error::RomstError, filesystem::FileChecks};
use super::models::{file::DataFile, game::Game, set::GameSet};
use anyhow::Result;
use console::Style;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DbDataFile {
    pub id: u32,
    pub file: DataFile,
}

impl DbDataFile {
    pub fn new(id: u32, file: DataFile) -> Self { Self { id, file } }
}


impl Display for DbDataFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.id, self.file)
    }
}

#[derive(Debug)]
pub struct RomSearch {
    searched_roms: HashSet<Rc<DbDataFile>>,
    pub set_results: HashMap<String, SetContent>,
    pub unknowns: Vec<DataFile>
}

impl RomSearch {
    pub fn new() -> Self {
        Self { searched_roms: HashSet::new(), set_results: HashMap::new(), unknowns: vec![] }
    }
    pub fn add_file_for_set(&mut self, set_name: String, file: DbDataFile) {
        let set_results = &mut self.set_results;

        let f = Rc::new(file);
        set_results.entry(set_name).or_insert(SetContent::new()).roms_included.insert(Rc::clone(&f));
        self.searched_roms.insert(f);
    }

    pub fn add_file_unknown(&mut self, file: DataFile) {
        self.unknowns.push(file);
    }

    pub fn get_roms_available_for_set(&self, set: &String) -> Vec<DbDataFile> {
        if let Some(set_result) = self.set_results.get(set){
            set_result.roms_included.iter().map(|data_file| {
                data_file.deref().to_owned()
            }).collect::<Vec<_>>()
        } else {
            vec![]
        }
    }

    pub fn get_roms_to_spare_for_set(&self, set: &String) -> Vec<DataFile> {
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
    pub fn get_searched_roms<'a>(&self) -> Vec<DataFile> {
        let a = self.searched_roms.iter().map(|item| {
            item.file.to_owned()
        }).collect::<Vec<_>>();
        a
    }
}

#[derive(Debug)]
pub struct SetContent {
    roms_included: HashSet<Rc<DbDataFile>>
}

impl SetContent {
    fn new() -> Self { Self { roms_included: HashSet::new() } }

    pub fn get_roms_included(&self) -> Vec<&DbDataFile> {
        let a = self.roms_included.iter().map(|data_file| {
            data_file.deref()
        }).collect::<Vec<_>>();
        a
    }
}

impl  Display for RomSearch {
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
    /// Returns all the roms for a specific romset
    fn get_romset_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<Vec<DbDataFile>> where S: AsRef<str> + rusqlite::ToSql;
    fn get_game_set<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<GameSet> where S: AsRef<str> + rusqlite::ToSql {
        match self.get_game(&game_name) {
            Some(game) => {
                let roms = self.get_romset_roms(game_name, rom_mode)?.into_iter().map(|db_rom| {
                    db_rom.file
                }).collect();
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
    use super::{DbDataFile, FileCheckSearch, RomSearch};
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
        rom_search.add_file_for_set("set1".to_string(), DbDataFile::new(1, rom1));
        rom_search.add_file_for_set("set2".to_string(), DbDataFile::new(2, rom2));
        rom_search.add_file_for_set("set2".to_string(), DbDataFile::new(3, rom3));

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