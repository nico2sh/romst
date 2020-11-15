use std::fmt::Display;

use super::{file::DataFile, game::Game};

pub struct GameSet {
    pub game: Game,
    pub roms: Vec<DataFile>,
    pub samples: Vec<DataFile>,
    pub disks: Vec<DataFile>,
}

impl GameSet {
    pub fn new(game: Game, roms: Vec<DataFile>, samples: Vec<DataFile>, disks: Vec<DataFile>) -> Self { Self { game, roms, samples, disks } }
}


impl Display for GameSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = format!("{}", self.game);
        for rom in self.roms.as_slice() {
            output.push_str(&format!("\n    - {}", rom));
        }
        for sample in self.samples.as_slice() {
            output.push_str(&format!("\n    - {}", sample));
        }
        for disk in self.disks.as_slice() {
            output.push_str(&format!("\n    - {}", disk));
        }

        write!(f, "{}", output)
    }
}