use std::fmt::{self, Display};

use super::file::DataFile;

#[derive(Debug)]
pub struct Report {
    pub name: String,
    pub roms_have: Vec<DataFile>,
    pub roms_to_rename: Vec<FileRename>,
    pub roms_missing: Vec<DataFile>,
    pub roms_unneeded: Vec<DataFile>,
}

#[derive(Debug)]
pub struct FileRename {
    pub from: DataFile,
    pub to: String,
}

impl Report {
    pub fn new(name: String, roms_have: Vec<DataFile>, roms_to_rename: Vec<FileRename>, roms_missing: Vec<DataFile>, roms_unneeded: Vec<DataFile>) -> Self {
        Self { name, roms_have, roms_to_rename, roms_missing, roms_unneeded }
    }

    pub fn empty(name: String) -> Self {
        Self {
            name,
            roms_have: vec![],
            roms_to_rename: vec![],
            roms_missing: vec![],
            roms_unneeded: vec![],
        }
    }

    pub fn add_having(&mut self, rom: DataFile) {
        self.roms_have.push(rom);
    }

    pub fn add_missing(&mut self, rom: DataFile) {
        self.roms_missing.push(rom);
    }
}


impl Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = format!("{}", self.name);
        if self.roms_have.len() > 0 {
            output.push_str("\nRoms:");
            for have in self.roms_have.as_slice() {
                output.push_str(&format!("\n    - {}", have));
            }
        }

        if self.roms_to_rename.len() > 0 {
            output.push_str("\nTo Rename:");
            for to_rename in self.roms_to_rename.as_slice() {
                output.push_str(&format!("\n    - {} => {}", to_rename.from, to_rename.to));
            }
        }

        if self.roms_missing.len() > 0 {
            output.push_str("\nMissing:");
            for missing in self.roms_missing.as_slice() {
                output.push_str(&format!("\n    - {}", missing));
            }
        }

        if self.roms_unneeded.len() > 0 {
            output.push_str("\nUnneeded:");
            for unneeded in self.roms_unneeded.as_slice() {
                output.push_str(&format!("\n    - {}", unneeded));
            }
        }

        write!(f, "{}", output)
    }
}