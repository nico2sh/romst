use indicatif::{ProgressBar, ProgressStyle};

use crate::data::importer::DatImporterReporter;

#[derive(Debug)]
pub struct DatImporterReporterSysOut {
    progress_bar: ProgressBar,
    entries: u32,
}

impl DatImporterReporterSysOut {
    pub fn new() -> Self { 
        let progress_bar = ProgressBar::new_spinner();
        Self { progress_bar, entries: 0 }
    }
}

impl DatImporterReporter for DatImporterReporterSysOut {
    fn set_total_bytes(&mut self, total_bytes: u64) {
        self.progress_bar.set_length(total_bytes);
        self.progress_bar.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/blue}] {bytes}/{total_bytes} ({eta}) | {msg}")
            .progress_chars("#>-"));
    }

    fn update_position(&mut self, current_bytes: u64, new_entries: u32) {
        self.entries = self.entries + new_entries;

        self.progress_bar.set_position(current_bytes);
        self.progress_bar.set_message(&format!("Entries: #{}", self.entries));
    }

    fn start_finish(&self) {
        self.progress_bar.finish_at_current_pos();
        self.progress_bar.set_message(&format!("Finishing, hold on..."));
    }

    fn finish(&self) {
        self.progress_bar.set_message(&format!("Entries: #{}", self.entries));
        self.progress_bar.finish_with_message(&format!("Total Entries #{}", self.entries));
    }
}