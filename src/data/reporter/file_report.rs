use std::fmt::{self, Display};
use console::Style;
use serde::{Deserialize, Serialize};
use crate::{RomsetMode, data::models::file::DataFile};

#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    rom_mode: RomsetMode,
    pub files: Vec<FileReport>,
}

impl Report {
    pub fn new(rom_mode: RomsetMode) -> Self { Self { rom_mode, files: vec![] } }

    pub fn from_files(rom_mode: RomsetMode, files: Vec<FileReport>) -> Self {
        Self { rom_mode, files }
    }

    pub fn add_set(&mut self, file_report: FileReport) {
        self.files.push(file_report);
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for file in &self.files {
            write!(f, "{}", file)?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileReport {
    pub rom_mode: RomsetMode,
    pub file_name: String,
    pub sets: Vec<SetReport>,
    pub unknown: Vec<DataFile>
}

impl FileReport {
    pub fn new<S>(file_name: S, rom_mode: RomsetMode) -> Self where S: Into<String> {
        Self { file_name: file_name.into(), rom_mode, sets: vec![], unknown: vec![] }
    }
    pub fn add_set(&mut self, set: SetReport) {
        self.sets.push(set);
    }
    pub fn add_unknown(&mut self, unknown: DataFile) {
        self.unknown.push(unknown);
    }
    pub fn get_full_sets(&self) -> Vec<&SetReport> {
        self.sets.iter().filter(|set| {
            set.roms_missing.len() == 0 && set.roms_to_rename.len() == 0
        }).collect::<Vec<_>>()
    }
    pub fn get_fixeable_sets(&self) -> Vec<&SetReport> {
        self.sets.iter().filter(|set| {
            set.roms_missing.len() == 0 && set.roms_to_rename.len() > 0
        }).collect::<Vec<_>>()
    }
    pub fn get_uncomplete_sets(&self) -> Vec<&SetReport> {
        self.sets.iter().filter(|set| {
            set.roms_missing.len() > 0
        }).collect::<Vec<_>>()
    }
}

impl Display for FileReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", Style::new().bold().yellow().apply_to("File name"), self.file_name)?;
        for s in &self.sets {
            writeln!(f, "- {}: {}", Style::new().blue().apply_to("Set"), s)?;
        }
        if self.unknown.len() > 0 {
            writeln!(f, "{}", Style::new().red().apply_to("Unknown files:"))?;
        }
        for file in &self.unknown {
            writeln!(f, "- {}", file)?;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetReport {
    pub name: String,
    pub roms_have: Vec<DataFile>,
    pub roms_to_rename: Vec<FileRename>,
    pub roms_missing: Vec<DataFile>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileRename {
    pub from: DataFile,
    pub to: String,
}

impl FileRename {
    pub fn new(from: DataFile, to: String) -> Self { Self { from, to } }
}

impl SetReport {
    pub fn new(name: String) -> Self {
        Self {
            name,
            roms_have: vec![],
            roms_to_rename: vec![],
            roms_missing: vec![],
        }
    }

    pub fn from_data(name: String, roms_have: Vec<DataFile>, roms_to_rename: Vec<FileRename>, roms_missing: Vec<DataFile>) -> Self {
        Self { name, roms_have, roms_to_rename, roms_missing }
    }

    pub fn add_having(&mut self, rom: DataFile) {
        self.roms_have.push(rom);
    }

    pub fn add_missing(&mut self, rom: DataFile) {
        self.roms_missing.push(rom);
    }
}

impl Display for SetReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.name)?;

        if self.roms_have.len() > 0 {
            writeln!(f, "{}", Style::new().cyan().apply_to("Roms:"))?;
            for have in self.roms_have.as_slice() {
                writeln!(f, "    - {}", have)?;
            }
        }

        if self.roms_to_rename.len() > 0 {
            writeln!(f, "{}", Style::new().magenta().apply_to("To Rename:"))?;
            for to_rename in self.roms_to_rename.as_slice() {
                writeln!(f, "    - {} => {}", to_rename.from, to_rename.to)?;
            }
        }

        if self.roms_missing.len() > 0 {
            writeln!(f, "{}", Style::new().red().apply_to("Missing:"))?;
            for missing in self.roms_missing.as_slice() {
                writeln!(f, "    - {}", missing)?;
            }
        }

        Ok(())
    }
}