use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug)]
pub struct DatReaderReporter {
    progress_bar: ProgressBar,
    entries: usize,
}

impl DatReaderReporter {
    pub fn new(total_bytes: u64) -> Self { 
        let progress_bar = ProgressBar::new(total_bytes);
        progress_bar.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/blue}] {bytes}/{total_bytes} ({eta}) | {msg}")
            .progress_chars("#>-"));   
        Self { progress_bar, entries: 0 }
    }

    pub fn update_position(&mut self, bytes: u64) {
        self.entries = self.entries + 1;

        self.progress_bar.set_position(bytes);
        self.progress_bar.set_message(&format!("Entries: #{}", self.entries));
    }

    pub fn finish(&self) {
        self.progress_bar.finish_with_message(&format!("Total Entries #{}", self.entries));
    }
}

