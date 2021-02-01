use std::collections::HashMap;

use crate::data::models::{self, file::DataFile};

use super::file_report::FileReport;

pub struct ScanReport {
    pub complete: HashMap<String, Set>,
    pub incomplete: HashMap<String, Set>,
}

impl ScanReport {
    pub fn new() -> Self { Self { complete: HashMap::new(), incomplete: HashMap::new() } }

    pub fn add_file_report(file_report: FileReport) {
        for set in file_report.get_full_sets() {
            
        }
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
}