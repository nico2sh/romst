use std::collections::HashMap;

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
    pub roms_available: Vec<SetRom>,
    pub roms_missing: Vec<DataFile>,
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
            roms_available: vec![],
            roms_missing: vec![],
        }
    }

    pub fn is_complete(&self) -> SetStatus {
        if self.roms_missing.len() == 0 {
            let available = self.roms_available.len();
            let mut files_count = HashMap::new();

            for set_rom in &self.roms_available {
                for location in &set_rom.location {
                    let number = files_count.entry(&location.file).or_insert(0);
                    if location.with_name.eq(&set_rom.file.name) {
                        *number += 1;
                    }
                }
            }

            for entry in files_count {
                if entry.1.eq(&available) {
                    let file_name = entry.0;
                    if models::does_file_belong_to_set(file_name, &self.name) {
                        return SetStatus::COMPLETE;
                    }
                }
            }

            return SetStatus::FIXEABLE;
        } else {
            SetStatus::INCOMPLETE
        }
    }

    pub fn add_set_rom(&mut self, location: RomLocation, file: DataFile) {
        let found = self.find_in_missing(&file);
        if let Some(pos) = found {
            self.roms_missing.remove(pos);
        }

        match self.roms_available.iter().position(|d| {
            d.file.eq(&file)
        }) {
            Some(index) => {
                self.roms_available[index].location.push(location);
            }
            None => {
                self.roms_available.push(SetRom::new(location, file));
            }
        };
    }

    pub fn add_missing_rom(&mut self, file: DataFile) {
        let found_unavailable = self.find_in_missing(&file);
        let found_available = self.find_in_available(&file);

        if found_available == None && found_unavailable == None {
            self.roms_missing.push(file);
        }
    }

    fn find_in_available(&self, file: &DataFile) -> Option<usize> {
        self.roms_available.iter().position(|set_rom| {
            if let Ok(comp) = file.info.deep_compare(&set_rom.file.info, FileChecks::ALL) {
                return comp;
            } else {
                return false;
            }
        })
    }

    fn find_in_missing(&self, file: &DataFile) -> Option<usize> {
        self.roms_missing.iter().position(|datafile| {
            if let Ok(comp) = file.info.deep_compare(&datafile.info, FileChecks::ALL) {
                return comp;
            } else {
                return false;
            }
        })
    }
}

pub struct SetRom {
    pub location: Vec<RomLocation>,
    pub file: DataFile,
}

impl SetRom {
    pub fn new(location: RomLocation, file: DataFile) -> Self { Self { location: vec![location], file } }
}


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