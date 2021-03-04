use std::fmt;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    pub clone_of: Option<String>,
    pub rom_of: Option<String>,
    pub source_file: Option<String>,
    pub sample_of: Option<String>,
    pub info_description: Option<String>,
    pub info_year: Option<String>,
    pub info_manufacturer: Option<String>,
}

impl Game {
    pub fn new(name: String) -> Self {
        Self {
            name,
            clone_of: None,
            rom_of: None,
            source_file: None,
            sample_of: None,
            info_description: None,
            info_year: None,
            info_manufacturer: None
        }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut game_data = vec![];

        if let Some(clone_of) = &self.clone_of {
            game_data.push(format!("Clone of: {}", clone_of));
        }
        if let Some(rom_of) = &self.rom_of {
            game_data.push(format!("ROM of: {}", rom_of));
        }
        if let Some(source_file) = &self.source_file {
            game_data.push(format!("Source File: {}", source_file));
        }
        if let Some(sample_of) = &self.sample_of {
            game_data.push(format!("Sample of: {}", sample_of))
        }

        let name_and_desc = match self.info_description {
            Some(ref desc) => { format!("[{}] {}", self.name, desc) }
            None => { format!("[{}]", self.name) }
        };

        write!(f, "{} ({})", name_and_desc, game_data.join(", "))
    }
}