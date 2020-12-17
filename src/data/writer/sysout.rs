use crate::{sysout::SysOutWriterReporter, data::models::{file::DataFile, game::Game}};
use anyhow::Result;

use super::DataWriter;

#[derive(Debug)]
pub struct SysOutWriter {
    reporter: SysOutWriterReporter,
}

impl SysOutWriter {
    pub fn new() -> Self { Self { reporter: SysOutWriterReporter::new() } }
}

impl DataWriter for SysOutWriter {
    fn init(&self) -> Result<()> {
        // println!("Initializing...");
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }

    fn on_new_entry(&mut self, game: Game, roms: Vec<DataFile>, disks: Vec<DataFile>, samples: Vec<String>, device_refs: Vec<String>) -> Result<()> {
        self.reporter.current_game(&game.name);
        for rom in roms {
            self.reporter.current_rom(&rom.name);
        };
        todo!()
    }
}
