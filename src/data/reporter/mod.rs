use crate::{RomsetMode, filesystem::{FileReader, FileChecks}};

use super::{reader::DataReader, models::report::Report};
use anyhow::Result;


pub struct Reporter<R: DataReader> {
    data_reader: R,
    file_reader: FileReader,
}

impl<R: DataReader> Reporter<R> {
    pub fn new(data_reader: R, file_reader: FileReader) -> Self { Self { data_reader, file_reader } }

    pub fn check_file(&mut self, file_path: &String, rom_mode: &RomsetMode) -> Result<Report> {
        let game_set = self.file_reader.get_game_set(file_path, FileChecks::ALL)?;

        let game = self.data_reader.get_game(&game_set.game.name)?;
        let roms = self.data_reader.get_romset_roms(&game_set.game.name, rom_mode)?;

        let report = Report::new(game.name, vec![], vec![]);

        Ok(report)
    }
}
