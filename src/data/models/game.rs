use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Game {
    pub name: String,
    pub clone_of: Option<String>,
    pub rom_of: Option<String>,
    pub source_file: Option<String>,
    pub info_description: Option<String>,
    pub info_year: Option<String>,
    pub info_manufacturer: Option<String>,
}

impl Game {
    pub fn new(name: String) -> Self { Self { name, clone_of: None, rom_of: None, source_file: None, info_description: None, info_year: None, info_manufacturer: None } }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut game_data = vec![];
        match self.clone_of {
            Some(ref clone_of) => game_data.push(format!("sha1: {}", clone_of)),
            _ => (),
        }
        match self.rom_of {
            Some(ref rom_of) => game_data.push(format!("md5: {}", rom_of)),
            _ => (),
        }
        match self.source_file {
            Some(ref source_file) => game_data.push(format!("crc: {}", source_file)),
            _ => (),
        }

        write!(f, "Game Name: {} ({})", self.name, game_data.join(", "))
    }
}