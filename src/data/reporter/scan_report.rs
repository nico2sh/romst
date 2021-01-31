use std::collections::HashMap;

use crate::data::models::file::DataFile;

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

enum SetStatus {
    COMPLETE,
    FIXEABLE,
    INCOMPLETE
}

impl Set {
    pub fn new<S>(name: S) -> Self where S: Into<String> {
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
            if let Some(count) = files_count.get(&self.name) {
                if count.eq(&available) {
                    SetStatus::COMPLETE
                } else {
                    SetStatus::FIXEABLE
                }
            } else {
                SetStatus::FIXEABLE
            }
        } else {
            SetStatus::INCOMPLETE
        }
    }

    pub fn add_set_rom(location: String, )
}

pub struct SetRom {
    pub location: Vec<RomLocation>,
    pub file: DataFile,
}

pub struct RomLocation {
    file: String,
    with_name: String,
}

impl RomLocation {
    pub fn new(file: String, with_name: String) -> Self { Self { file, with_name } }
}


#[cfg(test)]
mod tests {
    use crate::data::models::file::{DataFileInfo, FileType};

    use super::*;

    #[test]
    fn has_complete_set() {
        let set = Set::new("test");
        set.roms_available.push("file1", DataFile::new("file1", DataFileInfo::new(FileType::Rom)));
        let file1 = DataFile::new("file1", DataFileInfo::new(FileType::Rom));
        let file1 = DataFile::new("file2", DataFileInfo::new(FileType::Rom));
        let file1 = DataFile::new("file3", DataFileInfo::new(FileType::Rom));
    }
}