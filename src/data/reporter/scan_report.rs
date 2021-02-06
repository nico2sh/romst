use std::collections::{HashMap, HashSet};

use crate::data::models::{self, file::DataFile};

use crate::filesystem::FileChecks;
use super::file_report::FileReport;

pub struct ScanReport {
    pub sets: HashMap<String, Set>,
}

impl ScanReport {
    pub fn new() -> Self { Self { sets: HashMap::new() } }

    pub fn add_file_report(&mut self, file_report: FileReport) {
        let file_name = &file_report.file_name;
        file_report.sets.into_iter().for_each(|set_report| {
            let set_name = set_report.name.as_str();
            let set = self.sets.entry(set_name.to_string()).or_insert(Set::new(set_name));
            set_report.roms_have.into_iter().for_each(|rom| {
                set.add_set_rom(RomLocation::new(file_name, &rom.name), rom);
            });
            set_report.roms_to_rename.into_iter().for_each(|file_rename| {
                let rom = DataFile::new(file_rename.to, file_rename.from.info);
                set.add_set_rom(RomLocation::new(file_name, &file_rename.from.name), rom);
            });
            set_report.roms_missing.into_iter().for_each(|missing| {
                set.add_missing_rom(missing);
            });
        });
    }
}

pub struct Set {
    pub name: String,
    pub roms_available: HashMap<DataFile, RomLocatedAt>,
    pub roms_missing: HashSet<DataFile>,
    pub roms_to_spare: HashSet<DataFile>
}

#[derive(Debug, PartialEq)]
pub enum RomLocatedAt {
    InSet,
    InSetWrongName(String),
    InOthers(Vec<RomLocation>)
}

#[derive(Debug, PartialEq)]
enum SetStatus {
    COMPLETE,
    FIXEABLE,
    INCOMPLETE
}

impl Set {
    fn new<S>(name: S) -> Self where S: Into<String> {
        Self {
            name: name.into(),
            roms_available: HashMap::new(),
            roms_missing: HashSet::new(),
            roms_to_spare: HashSet::new(),
        }
    }

    pub fn is_complete(&self) -> SetStatus {
        if self.roms_missing.len() == 0 {
            let mut available = self.roms_available.len();

            for set_rom in &self.roms_available {
                if set_rom.1.eq(&RomLocatedAt::InSet) {
                    available -= 1;
                }
            }

            if available == 0 {
                return SetStatus::COMPLETE;
            }

            return SetStatus::FIXEABLE;
        } else {
            SetStatus::INCOMPLETE
        }
    }

    pub fn add_set_rom(&mut self, location: RomLocation, file: DataFile) {
        let found = self.find_in_missing(&file);
        if found {
            self.roms_missing.remove(&file);
        }

        let in_set = models::does_file_belong_to_set(location.file.as_str(), self.name.as_str());
        let file_name = file.name.clone();
        match self.roms_available.entry(file) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if let RomLocatedAt::InOthers(ref mut locations) = entry.get_mut() {
                    if !in_set {
                        locations.push(location);
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let av = if in_set {
                    if location.with_name.eq(&file_name) {
                        RomLocatedAt::InSet
                    } else {
                        RomLocatedAt::InSetWrongName(file_name)
                    }
                } else {
                    RomLocatedAt::InOthers(vec![location])
                };

                entry.insert(av);
            }
        }
    }

    pub fn add_missing_rom(&mut self, file: DataFile) {
        let found_unavailable = self.find_in_missing(&file);
        let found_available = self.find_in_available(&file);

        if !found_available || !found_unavailable {
            self.roms_missing.insert(file);
        }
    }

    fn find_in_available(&self, file: &DataFile) -> bool {
        self.roms_available.get(file).is_some()
    }

    fn find_in_missing(&self, file: &DataFile) -> bool {
        self.roms_missing.get(file).is_some()
    }
}

#[derive(Debug, PartialEq)]
pub struct RomLocation {
    file: String,
    with_name: String,
}

impl RomLocation {
    pub fn new<S>(file: S, with_name: S) -> Self where S: Into<String> { Self { file: file.into(), with_name: with_name.into() } }
}


#[cfg(test)]
mod tests {
    use crate::data::models::file::{DataFileInfo, FileType};

    use super::*;

    #[test]
    fn has_complete_set() {
        let mut set = Set::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", DataFileInfo::new(FileType::Rom)));

        set.add_set_rom(RomLocation::new("set1.zip", "file2"),
            DataFile::new("file2", DataFileInfo::new(FileType::Rom)));

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", DataFileInfo::new(FileType::Rom)));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    #[test]
    fn has_fixeable_set_with_files_from_another_set() {
        let mut set = Set::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", DataFileInfo::new(FileType::Rom)));

        set.add_set_rom(RomLocation::new("set2.zip", "file2"),
            DataFile::new("file2", DataFileInfo::new(FileType::Rom)));

        set.add_set_rom(RomLocation::new("set3.zip", "file3"),
            DataFile::new("file3", DataFileInfo::new(FileType::Rom)));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::FIXEABLE, completeness);
    }

    #[test]
    fn has_fixeable_set_with_files_to_rename() {
        let mut set = Set::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", DataFileInfo::new(FileType::Rom)));

        set.add_set_rom(RomLocation::new("set1.zip", "file_wrong_name"),
            DataFile::new("file2", DataFileInfo::new(FileType::Rom)));

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", DataFileInfo::new(FileType::Rom)));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::FIXEABLE, completeness);
    }

    #[test]
    fn has_incomplete_set() {
        let mut set = Set::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", DataFileInfo::new(FileType::Rom)));
        set.add_set_rom(RomLocation::new("set1.zip", "file2"),
            DataFile::new("file2", DataFileInfo::new(FileType::Rom)));
        set.add_missing_rom(DataFile::new("file3", get_sample_rom("8bb3a81b9fa2de5163f0ffc634a998c455bcca25")));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::INCOMPLETE, completeness);

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", get_sample_rom("8bb3a81b9fa2de5163f0ffc634a998c455bcca25")));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    fn get_sample_rom<S>(sha1: S) -> DataFileInfo where S: Into<String>{
        let mut rom = DataFileInfo::new(FileType::Rom);
        rom.sha1 = Some(sha1.into());
        rom
    }
}