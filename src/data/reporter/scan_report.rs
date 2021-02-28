use std::{collections::{HashMap, HashSet, hash_map::Entry}, fmt::Display};

use log::debug;

use crate::{RomsetMode, data::models::{self, file::DataFile, game::Game}};

#[derive(Debug)]
pub struct ScanReport {
    root_directory: Option<String>,
    rom_mode: RomsetMode,
    pub sets: HashMap<String, SetReport>,
    pub ignored: Vec<String>,
}

impl Display for ScanReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.root_directory {
            writeln!(f, "Scanned dir: {}", path)?;
        }
        writeln!(f, "Mode: {}", self.rom_mode)?;
        writeln!(f)?;
        if !self.ignored.is_empty() {
            writeln!(f, "Ignored:")?;
            for file in &self.ignored {
                writeln!(f, "- {}", file)?;
            }
            writeln!(f)?;
        }

        for set in &self.sets {
            let s = set.1; 
            writeln!(f, "{}", s)?;
        }
        Ok(())
    }
}

impl ScanReport {
    pub fn new(root_directory: Option<String>, rom_mode: RomsetMode) -> Self {
        Self {
            root_directory,
            rom_mode, sets: HashMap::new(),
            ignored: vec![]
        }
    }

    pub fn add_ignored<S>(&mut self, file: S) where S: Into<String> {
        self.ignored.push(file.into());
    }

    pub fn add_rom_for_set<S>(&mut self, set_name: S, location: RomLocation, rom: DataFile) where S: AsRef<str> {
        let set = self.sets.entry(set_name.as_ref().to_owned()).or_insert_with(|| SetReport::new(set_name.as_ref()));
        match &rom.status {
            Some(status) if status.to_lowercase() == "nodump" => {
                set.roms_unneeded.insert(rom);
            }
            _ => {
                set.add_set_rom(location, rom);
            }
        }
    }

    pub fn add_missing_roms_for_set<I, S>(&mut self, set_name: S, roms: I) where I: IntoIterator<Item = DataFile>, S: AsRef<str> {
        let set = self.sets.entry(set_name.as_ref().to_owned()).or_insert_with(|| SetReport::new(set_name.as_ref()));
        roms.into_iter().for_each(|rom| {
            match &rom.status {
                Some(status) if status.to_lowercase() == "nodump" => {
                    set.roms_unneeded.insert(rom);
                }
                _ => {
                    set.add_missing_rom(rom);
                }
            }
        });
    }

    pub fn add_missing_rom_for_set<S>(&mut self, set_name: S, rom: DataFile) where S: AsRef<str> {
        let set = self.sets.entry(set_name.as_ref().to_owned()).or_insert_with(|| SetReport::new(set_name.as_ref()));
        match &rom.status {
            Some(status) if status.to_lowercase() == "nodump" => {
                set.roms_unneeded.insert(rom);
            }
            _ => {
                set.add_missing_rom(rom);
            }
        }
    }

    pub fn set_in_file<S>(&mut self, source_file: S) where S: AsRef<str> {
        let set_name = models::get_set_from_file(source_file.as_ref());
        self.sets.entry(set_name.clone()).or_insert_with(|| SetReport::new(set_name)).in_file = true;
    }

    pub fn add_unknown_files<I, S>(&mut self, files: I, source_file: S) where I: IntoIterator<Item = DataFile>, S: AsRef<str> {
        let set_name = models::get_set_from_file(source_file.as_ref());
        let set = self.sets.entry(set_name.clone()).or_insert_with(|| SetReport::new(set_name));
        for file in files {
            set.unknown.push(file);
        }
    }

    pub fn add_roms_to_spare<I, S>(&mut self, files: I, source_file: S) where I: IntoIterator<Item = DataFile>, S: AsRef<str> {
        let set_name = models::get_set_from_file(source_file.as_ref());
        let set = self.sets.entry(set_name.clone()).or_insert_with(|| SetReport::new(set_name));
        files.into_iter().for_each(|rom| {
            set.roms_to_spare.insert(rom);
        });
    }

    pub fn reference_with_game(&mut self, game: Game) {
        let set_name = &game.name;
        let set = self.sets.entry(set_name.to_owned()).or_insert_with(|| SetReport::new(set_name));
        set.ref_game(game);
    }
}

#[derive(Debug)]
pub struct SetReport {
    pub reference: SetReference,
    pub in_file: bool,
    pub roms_available: HashMap<DataFile, RomLocatedAt>,
    pub roms_missing: HashSet<DataFile>,
    pub roms_unneeded: HashSet<DataFile>, // BadDumps
    pub roms_to_spare: HashSet<DataFile>,
    pub unknown: Vec<DataFile>
}

// A set may be associated with a game based on its name, or just contain roms if there are no matches
#[derive(Debug)]
pub enum SetReference {
    FileName(String),
    Game(Game)
}

impl SetReference {
    pub fn get_name(&self) -> &str {
        match self {
            SetReference::FileName(name) => {
                name
            }
            SetReference::Game(game) => {
                &game.name
            }
        }
    }
}

impl Display for SetReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetReference::FileName(name) => {
                writeln!(f, "File name: {}", name)
            }
            SetReference::Game(game) => {
                writeln!(f, "{}", game)
            }
        }
    }
}

impl Display for SetReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Set - {}", self.reference)?;
        let file_status = if self.in_file {
            " [in file]"
        } else {
            ""
        };
        writeln!(f, "Status: {}{}", self.is_complete(), file_status)?;
        if !self.roms_available.is_empty() {
            writeln!(f, "Roms Available")?;
            for available in &self.roms_available {
                let rom = available.0;
                let location = available.1;
                match location {
                    RomLocatedAt::InSet => { writeln!(f, " - {}", rom.name)?; }
                    RomLocatedAt::InSetWrongName(name) => { writeln!(f, " - {} [rename from: {}]", rom.name, name)?; }
                    RomLocatedAt::InOthers(locations) => {
                        let mut location_list = vec![];
                        for location in locations {
                            location_list.push(format!("{} as {}", location.file, location.with_name));
                        }
                        writeln!(f, " - {} [located at: {}]", rom.name, location_list.join(", "))?; 
                    }
                }
            }
        }
        if !self.roms_unneeded.is_empty() {
            writeln!(f, "Roms Unneeded (e.g. Bad Dumps)")?;
            for unneeded in &self.roms_unneeded {
                writeln!(f, " - {}", unneeded.name)?;
            }
        }
        if !self.roms_missing.is_empty() {
            writeln!(f, "Roms Missing")?;
            for missing in &self.roms_missing {
                writeln!(f, " - {}", missing.name)?;
            }
        }
        if !self.roms_to_spare.is_empty() {
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

impl SetReport {
    pub fn new<S>(name: S) -> Self where S: Into<String> {
        Self {
            reference: SetReference::FileName(name.into()),
            in_file: false,
            roms_available: HashMap::new(),
            roms_missing: HashSet::new(),
            roms_unneeded: HashSet::new(),
            roms_to_spare: HashSet::new(),
            unknown: vec![]
        }
    }

    pub fn ref_game(&mut self, game: Game) {
        self.reference = SetReference::Game(game)
    }

    pub fn is_complete(&self) -> SetStatus {
        if self.roms_missing.is_empty() {
            let mut available = self.roms_available.len();

            for set_rom in &self.roms_available {
                if set_rom.1.eq(&RomLocatedAt::InSet) {
                    available -= 1;
                }
            }

            if available == 0 {
                return SetStatus::COMPLETE;
            }

            SetStatus::FIXEABLE
        } else {
            SetStatus::INCOMPLETE
        }
    }

    fn add_set_rom(&mut self, location: RomLocation, rom: DataFile) {
        if self.roms_missing.remove(&rom) {
            debug!("Removed from set {} the file as missing {}", self.reference, &rom);
        }

        let in_set = models::does_file_belong_to_set(location.file.as_str(), self.reference.get_name());
        let rom_name = rom.name.clone();
        match self.roms_available.entry(rom) {
            Entry::Occupied(mut entry) => {
                match entry.get_mut() {
                    RomLocatedAt::InSet => {
                        // We don't do anything, we already have the file
                    },
                    RomLocatedAt::InSetWrongName(_name) => {
                        if in_set {
                            if location.with_name == rom_name {
                                entry.insert(RomLocatedAt::InSet);
                            }
                        } else {
                            // Nothing, it's in another place, we prioritize the same file
                        }
                    },
                    RomLocatedAt::InOthers(ref mut locations) => {
                        if in_set {
                            if location.with_name == rom_name {
                                entry.insert(RomLocatedAt::InSet);
                            } else {
                                entry.insert(RomLocatedAt::InSetWrongName(location.with_name));
                            }
                        } else {
                            locations.push(location);
                        }
                    }
                }
            }
            Entry::Vacant(entry) => {
                let av = if in_set {
                    if location.with_name == rom_name {
                        RomLocatedAt::InSet
                    } else {
                        RomLocatedAt::InSetWrongName(location.with_name)
                    }
                } else {
                    RomLocatedAt::InOthers(vec![location])
                };

                entry.insert(av);
            }
        }
    }

    fn add_missing_rom(&mut self, file: DataFile) {
        match &file.status {
            Some(status) if status.to_lowercase() == "nodump" => {
                self.roms_unneeded.insert(file);
            }
            _ => {
                let found_in_missing = self.find_in_missing(&file);
                let found_available = self.find_in_available(&file);

                if !found_available && !found_in_missing {
                    self.roms_missing.insert(file);
                }
            }
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
        let mut set = SetReport::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", get_sample_rom("1234")));

        set.add_set_rom(RomLocation::new("set1.zip", "file2"),
            DataFile::new("file2", get_sample_rom("5678")));

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", get_sample_rom("3456")));

        let completeness = set.is_complete();
        assert_eq!(3, set.roms_available.len());
        assert_eq!(0, set.roms_missing.len());
        assert_eq!(0, set.roms_to_spare.len());
        assert_eq!(0, set.unknown.len());
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    #[test]
    fn repeated_roms_with_different_names() {
        let mut set = SetReport::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", get_sample_rom("1234")));

        set.add_set_rom(RomLocation::new("set1.zip", "file2"),
            DataFile::new("file2", get_sample_rom("1234")));

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", get_sample_rom("3456")));

        let completeness = set.is_complete();
        assert_eq!(3, set.roms_available.len());
        assert_eq!(0, set.roms_missing.len());
        assert_eq!(0, set.roms_to_spare.len());
        assert_eq!(0, set.unknown.len());
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    #[test]
    fn has_fixeable_set_with_files_from_another_set() {
        let mut set = SetReport::new("set1");
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
        let mut set = SetReport::new("set1");
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
    fn has_incomplete_set_then_complete() {
        let mut set = SetReport::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", DataFileInfo::new(FileType::Rom)));
        set.add_set_rom(RomLocation::new("set1.zip", "file2"),
            DataFile::new("file2", DataFileInfo::new(FileType::Rom)));
        set.add_missing_rom(DataFile::new("file3", get_sample_rom("1234")));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::INCOMPLETE, completeness);

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", get_sample_rom("1234")));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    #[test]
    fn has_fixeable_then_complete() {
        let mut set = SetReport::new("set1");
        set.add_set_rom(RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", get_sample_rom("7890")));
        set.add_set_rom(RomLocation::new("set1.zip", "file2"),
            DataFile::new("file2", get_sample_rom("4567")));
        set.add_set_rom(RomLocation::new("set2.zip", "file3"),
            DataFile::new("file3", get_sample_rom("1234")));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::FIXEABLE, completeness);

        set.add_set_rom(RomLocation::new("set1.zip", "file3"),
            DataFile::new("file3", get_sample_rom("1234")));

        let completeness = set.is_complete();
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    #[test]
    fn has_missing_then_fixeable_then_complete() {
        let mut scan_report = ScanReport::new(None, RomsetMode::Split);

        // A rom in the right place
        scan_report.add_rom_for_set("set1", RomLocation::new("set1.zip", "file1"),
            DataFile::new("file1", get_sample_rom("1234")));
        // A rom with a different name
        scan_report.add_rom_for_set("set1", RomLocation::new("set1.zip", "dupfile2"),
            DataFile::new("dupfile1", get_sample_rom("1234")));
        // Same rom with the right name
        scan_report.add_rom_for_set("set1", RomLocation::new("set1.zip", "dupfile1"),
            DataFile::new("dupfile1", get_sample_rom("1234")));

        scan_report.add_rom_for_set("set1", RomLocation::new("set1.zip", "dupfile1"),
            DataFile::new("dupfile2", get_sample_rom("1234")));
        // Same rom with the right name
        scan_report.add_rom_for_set("set1", RomLocation::new("set1.zip", "dupfile2"),
            DataFile::new("dupfile2", get_sample_rom("1234")));

        println!("{}", scan_report);

        let set = scan_report.sets.get("set1").unwrap();
        let completeness = set.is_complete();
        assert_eq!(SetStatus::COMPLETE, completeness);
    }

    fn get_sample_rom<S>(sha1: S) -> DataFileInfo where S: Into<String>{
        let mut rom = DataFileInfo::new(FileType::Rom);
        rom.sha1 = Some(sha1.into());
        rom
    }
}