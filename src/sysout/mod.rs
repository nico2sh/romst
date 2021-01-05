use std::thread;

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

use crate::data::{importer::DatImporterReporter, reporter::ReportReporter};

#[derive(Debug)]
pub struct DatImporterReporterSysOut {
    progress_bar: ProgressBar,
    entries: u32,
}

impl DatImporterReporterSysOut {
    pub fn new() -> Self { 
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_draw_target(ProgressDrawTarget::stdout());
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

pub struct ReportReporterSysOut {
    progress_bar: ProgressBar,
    total_files: usize,
    current_files: usize,
    new_files: usize,
    directories: usize,
    ignored: usize,
    current_file: String,
}

impl ReportReporterSysOut {
    pub fn new() -> Self {
        let progress_bar =ProgressBar::new(!0);
        progress_bar.set_draw_target(ProgressDrawTarget::stdout());
        progress_bar.set_style(ProgressStyle::default_bar()
            .template("{prefix}\n{spinner:.green} [{elapsed_precise}] [{bar:40.green/blue}] {bytes}/{total_bytes} ({eta}) | {msg}")
            .progress_chars("#>-"));
        progress_bar.set_prefix("P: Processed / D: Directories / I: Ignored");
        Self { progress_bar, total_files: !0, current_files: 0, new_files: 0, directories: 0, ignored: 0, current_file: String::new() }
    }

    fn update_info_numbers(&mut self) {
        self.progress_bar.set_prefix(&format!("P: Processed / D: Directories / I: Ignored | {}", self.current_file));
        self.progress_bar.set_message(&format!("P: {} / D: {} / I: {}", self.new_files, self.directories, self.ignored));
    }
}

impl ReportReporter for ReportReporterSysOut {
    fn set_total_files(&mut self, total_files: usize) {
        self.total_files = total_files;
        self.progress_bar.set_length(100);
    }

    fn update_report_new_file(&mut self, new_file: &str) {
        self.current_file = new_file.to_string();
        self.current_files += 1;
        let current_progress = (self.current_files * 100) / self.total_files;
        self.progress_bar.set_position(current_progress as u64);
    }

    fn update_report_new_added_file(&mut self, new_files: usize) {
        self.new_files += new_files;
        self.update_info_numbers();
    }

    fn update_report_directory(&mut self, new_files: usize) {
        self.directories += new_files;
        self.update_info_numbers();
    }

    fn update_report_ignored(&mut self, new_files: usize) {
        self.directories += new_files;
        self.update_info_numbers();
    }

    fn update_report_file_error(&mut self, new_files: usize) {
        self.ignored += new_files;
        self.update_info_numbers();
    }

    fn finish(&mut self) {
        self.progress_bar.set_prefix("P: Processed / D: Directories / I: Ignored | FINISHED");
        self.progress_bar.finish_with_message(&format!("P: {} / D: {} / I: {}", self.new_files, self.directories, self.ignored));
    }
}