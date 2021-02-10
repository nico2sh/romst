use std::fmt::{self, Display};
use console::Style;
use serde::{Deserialize, Serialize};
use crate::data::models::file::DataFile;

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
    pub fn new<S>(name: S) -> Self where S: Into<String> {
        Self {
            name: name.into(),
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