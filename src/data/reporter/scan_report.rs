use std::{collections::{HashMap, HashSet, hash_map::Entry}, fmt::Display};

use crate::{RomsetMode, data::models::{self, file::DataFile}};

use super::set_report::SetReport;

#[derive(Debug)]
pub struct ScanReport {
    rom_mode: RomsetMode,
    pub sets: HashMap<String, Set>,
}

impl Display for ScanReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Mode: {}", self.rom_mode)?;
        writeln!(f, "")?;
        for set in &self.sets {
            let s = set.1; 
            writeln!(f, "{}", s)?;
        }
        Ok(())
    }
}

impl ScanReport {
    pub fn new(rom_mode: RomsetMode) -> Self { Self { rom_mode, sets: HashMap::new() } }

    pub fn add_set_report(&mut self, set_report: SetReport, source_file: &String) {
        let set_name = set_report.name.as_str();
        let set = self.sets.entry(set_name.to_string()).or_insert(Set::new(set_name));
        let roms_have = set_report.roms_have;
        roms_have.into_iter().for_each(|rom| {
            let rom_name = rom.name.clone();
            set.add_set_rom(RomLocation::new(source_file.to_owned(), rom_name), rom);
        });
        set_report.roms_to_rename.into_iter().for_each(|file_rename| {
            let rom = DataFile::new(file_rename.to, file_rename.from.info);
            set.add_set_rom(RomLocation::new(source_file.to_owned(), file_rename.from.name), rom);
        });
        set_report.roms_missing.into_iter().for_each(|missing| {
            set.add_missing_rom(missing);
        });
    }

    pub fn add_unknown_files<I>(&mut self, files: I, source_file: String) where I: IntoIterator<Item = DataFile> {
        let set_name = models::get_set_from_file(source_file.as_str());
        let set = self.sets.entry(set_name.clone()).or_insert(Set::new(set_name));
        for file in files {
            set.unknown.push(file);
        }
    }

    pub fn add_roms_to_spare<I>(&mut self, files: I, source_file: &String) where I: IntoIterator<Item = DataFile> {
        let set_name = models::get_set_from_file(source_file.as_str());
        let set = self.sets.entry(set_name.clone()).or_insert(Set::new(set_name));
        files.into_iter().for_each(|rom| {
            set.roms_to_spare.insert(rom);
        });
    }
}

#[derive(Debug)]
pub struct Set {
    pub name: String,
    pub roms_available: HashMap<DataFile, RomLocatedAt>,
    pub roms_missing: HashSet<DataFile>,
    pub roms_to_spare: HashSet<DataFile>,
    pub unknown: Vec<DataFile>
}

impl Display for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Set Name: {}", self.name)?;
        writeln!(f, "Status: {}", self.is_complete())?;
        if self.roms_available.len() > 0 {
            writeln!(f, "Roms Available")?;
            for available in &self.roms_available {
                let rom = available.0;
                let location = available.1;
                match location {
                    RomLocatedAt::InSet => { writeln!(f, " - {}", rom.name)?; }
                    RomLocatedAt::InSetWrongName(name) => { writeln!(f, " - {} (rename from: {})", rom.name, name)?; }
                    RomLocatedAt::InOthers(locations) => {
                        let mut location_list = vec![];
                        for location in locations {
                            location_list.push(format!("{} as {}", location.file, location.with_name));
                        }
                        writeln!(f, " - {} (located at: {})", rom.name, location_list.join(", "))?; 
                    }
                }
            }
        }
        if self.roms_missing.len() > 0 {
            writeln!(f, "Roms Missing")?;
            for missing in &self.roms_missing {
                writeln!(f, " - {}", missing.name)?;
            }
        }
        if self.roms_to_spare.len() > 0 {
            writeln!(f, "Roms to Spare")?;
            for to_spare in &self.roms_to_spare {
                writeln!(f, " - {}", to_spare.name)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum RomLocatedAt {
    InSet,
    InSetWrongName(String),
    InOthers(Vec<RomLocation>)
}

#[derive(Debug, PartialEq)]
pub enum SetStatus {
    COMPLETE,
    FIXEABLE,
    INCOMPLETE
}

impl Display for SetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetStatus::COMPLETE => {
                write!(f, "Complete")
            }
            SetStatus::FIXEABLE => {
                write!(f, "Fixeable")
            }
            SetStatus::INCOMPLETE => {
                write!(f, "Incomplete")
            }
        }
    }
}

impl Set {
    fn new<S>(name: S) -> Self where S: Into<String> {
        Self {
            name: name.into(),
            roms_available: HashMap::new(),
            roms_missing: HashSet::new(),
            roms_to_spare: HashSet::new(),
            unknown: vec![]
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
            Entry::Occupied(mut entry) => {
                if let RomLocatedAt::InOthers(ref mut locations) = entry.get_mut() {
                    if !in_set {
                        locations.push(location);
                    }
                }
            }
            Entry::Vacant(entry) => {
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
        let found_in_missing = self.find_in_missing(&file);
        let found_available = self.find_in_available(&file);

        if !found_available && !found_in_missing {
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