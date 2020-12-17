pub mod sqlite;
pub mod sysout;

use anyhow::Result;

use super::models::{game::Game, file::*};

pub trait DataWriter {
    fn init(&self) -> Result<()>;
    fn on_new_entry(&mut self, game: Game, roms: Vec<DataFile>, disks: Vec<DataFile>, samples: Vec<String>, device_refs: Vec<String>) -> Result<()>;
    fn finish(&mut self) -> Result<()>;
}