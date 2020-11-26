use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[derive(Debug)]
pub struct DatImporterReporter {
    progress_bar: ProgressBar,
    entries: u32,
}

impl DatImporterReporter {
    pub fn new(total_bytes: u64) -> Self { 
        let progress_bar = ProgressBar::new(total_bytes);
        progress_bar.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.green/blue}] {bytes}/{total_bytes} ({eta}) | {msg}")
            .progress_chars("#>-"));   
        Self { progress_bar, entries: 0 }
    }

    pub fn update_position(&mut self, bytes: u64, new_entries: u32) {
        self.entries = self.entries + new_entries;

        self.progress_bar.set_position(bytes);
        self.progress_bar.set_message(&format!("Entries: #{}", self.entries));
    }

    pub fn start_finish(&self) {
        self.progress_bar.set_message(&format!("Finishing..."));
    }

    pub fn finish(&self) {
        self.progress_bar.set_message(&format!("Entries: #{}", self.entries));
        self.progress_bar.finish_with_message(&format!("Total Entries #{}", self.entries));
    }
}

#[derive(Debug)]
pub struct SysOutWriterReporter {
    game_pb: ProgressBar,
    rom_pb: ProgressBar,
}

impl SysOutWriterReporter {
    pub fn new() -> Self {
        let spinner_style = ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{prefix:.bold.dim} {spinner} {wide_msg}");

        let multi_progress = MultiProgress::new();
        let game = multi_progress.add(ProgressBar::new_spinner());
        // game.set_style(spinner_style.clone());
        game.set_prefix("[Game]");
        let rom = multi_progress.add(ProgressBar::new_spinner());
        // rom.set_style(spinner_style.clone());
        rom.set_prefix("[ROM]");


        Self { game_pb: game, rom_pb: rom } 
    }

    pub fn current_game(&mut self, game: &str) {
        self.game_pb.set_message(&game);
        self.game_pb.inc(1);
    }

    pub fn current_rom(&mut self, rom: &str) {
        self.rom_pb.set_message(&rom);
        self.rom_pb.inc(1);
    }
}
