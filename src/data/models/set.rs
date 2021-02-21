use std::{fmt::Display, writeln};

use super::{file::DataFile, game::Game};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSet {
    pub game: Game,
    pub roms: Vec<DataFile>,
    pub samples: Vec<DataFile>,
    pub disks: Vec<DataFile>,
    pub device_refs: Vec<String>,
}

impl GameSet {
    pub fn new(game: Game, roms: Vec<DataFile>, samples: Vec<DataFile>, disks: Vec<DataFile>, device_refs: Vec<String>) -> Self { Self { game, roms, samples, disks, device_refs } }
}

impl Display for GameSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.game)?;
        if !self.roms.is_empty() {
            writeln!(f, "Roms:")?;
            for rom in self.roms.as_slice() {
                writeln!(f, "    - {}", rom)?;
            }
        }
        if !self.samples.is_empty() {
            writeln!(f, "Samples:")?;
            for sample in self.samples.as_slice() {
                writeln!(f, "    - {}", sample)?;
            }
        }
        if !self.disks.is_empty() {
            writeln!(f, "Disks:")?;
            for disk in self.disks.as_slice() {
                writeln!(f, "    - {}", disk)?;
            }
        }
        if !self.device_refs.is_empty() {
            writeln!(f, "Depends on:")?;
            for device_ref in self.device_refs.as_slice() {
                writeln!(f, "    - {}", device_ref)?;
            }
        }

        Ok(())
    }
}