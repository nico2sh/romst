pub mod report;

use std::path::{Path, PathBuf};
use crate::{RomsetMode, err, error::RomstIOError, filesystem::{FileReader, FileChecks}};
use self::report::{FileRename, FileReport, Report, SetReport};

use super::{models::{file::DataFile, set::GameSet}, reader::{self, DataReader}};
use anyhow::Result;
use crossbeam::sync::WaitGroup;
use tokio_stream::StreamExt;
use tokio::sync::mpsc::{Receiver, channel};
use log::{error, warn};

type RR = Option<Box<dyn ReportReporter>>;
    
pub struct Reporter<R: DataReader> {
    data_reader: R,
    reporter: RR
}

pub trait ReportReporter {
    fn set_total_files(&mut self, total_files: usize);
    fn update_report_new_file(&mut self, new_file: &str);
    fn update_report_new_added_file(&mut self, new_files: usize);
    fn update_report_directory(&mut self, new_files: usize);
    fn update_report_ignored(&mut self, new_files: usize);
    fn update_report_file_error(&mut self, new_files: usize);
    fn finish(&mut self);
}

enum ReportMessageContent {
    FoundGameSet(GameSet),
    FoundNotValid,
    FoundError,
    Done
}

struct ReportMessage {
    file_name: String, 
    content: ReportMessageContent
}

impl ReportMessage {
    fn new(file_name: String, content: ReportMessageContent) -> Self { Self { file_name, content } }
}

impl<R: DataReader> Reporter<R> {
    pub fn new(data_reader: R) -> Self { Self { data_reader, reporter: None } }

    pub fn add_reporter<P>(&mut self, reporter: P) where P: ReportReporter + 'static {
        self.reporter = Some(Box::new(reporter));
    }

    pub async fn check(&mut self, file_paths: Vec<impl AsRef<Path>>, rom_mode: RomsetMode) -> Result<Report> {
        if file_paths.len() == 1 {
            if let Some(path) = file_paths.get(0) {
                let p = path.as_ref();
                if p.is_dir() {
                    return self.check_directory(&p.to_path_buf(), rom_mode).await
                }
            }
        }

        self.check_files(file_paths, rom_mode).await
    }

    async fn check_directory(&mut self, file_path: &impl AsRef<Path>, rom_mode: RomsetMode) -> Result<Report> {
        let path = file_path.as_ref();
        if path.is_dir() {
            let contents = path.read_dir()?.into_iter().filter_map(|dir_entry| {
                if let Ok(ref entry) = dir_entry {
                    let path = entry.path();
                    Some(path)
                } else {
                    None
                }
            }).collect::<Vec<PathBuf>>();
            self.check_files(contents, rom_mode).await
        } else {
            err!("Path is not a directory")
        }
    }

    /// Returns a Receiver that will receive a message with the file reports.
    async fn send_sets_from_files(&mut self, file_paths: Vec<impl AsRef<Path>>) -> Result<Receiver<ReportMessage>> {
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.set_total_files(file_paths.len());
        }

        let (tx, receiver) = channel::<ReportMessage>(file_paths.len());
        let wg = WaitGroup::new();

        let file_checks = self.data_reader.get_file_checks()?;
        let mut use_checks = FileChecks::ALL;
        if file_checks.sha1 == 0 {
            use_checks = use_checks & !FileChecks::SHA1;
        }
        if file_checks.md5 == 0 {
            use_checks = use_checks & !FileChecks::MD5;
        }
        if file_checks.crc == 0 {
            use_checks = use_checks & !FileChecks::CRC;
        }

        file_paths.into_iter()
            .for_each(|fp| {
                let path = fp.as_ref();
                if path.is_file() {
                    let sender = tx.clone();
                    let p = path.to_path_buf();
                    let wg = wg.clone();

                    tokio::spawn(async move {
                        let file_name = match p.file_name() {
                            Some(file) => {
                                file.to_owned().into_string().unwrap_or_else(|os_string| {
                                    os_string.to_string_lossy().to_string()
                                })
                            }
                            None => { "UNKNOWN FILE".to_string() }
                        };

                        let mut file_reader = FileReader::new();
                        let result = match file_reader.get_game_set(&p, use_checks) {
                            Ok(game_set) => {
                                sender.send(ReportMessage::new(file_name,
                                    ReportMessageContent::FoundGameSet(game_set))).await
                            },
                            Err(RomstIOError::NotValidFileError(file_name, _file_type )) => {
                                sender.send(ReportMessage::new(file_name,
                                    ReportMessageContent::FoundNotValid)).await
                            },
                            Err(e) => {
                                error!("ERROR: {}", e);
                                sender.send(ReportMessage::new(file_name,
                                    ReportMessageContent::FoundError)).await
                            }
                        };

                        if let Err(error) = result {
                            error!("ERROR: {}", error);
                        }

                        drop(wg);
                    });
                } else {
                    if let Some(reporter) = self.reporter.as_mut() {
                        reporter.update_report_directory(1);
                    };
                }
            });
        let sender = tx.clone();
        tokio::spawn(async move {
            wg.wait();
            if let Err(error) = sender.send(ReportMessage::new("".to_string(), 
            ReportMessageContent::Done)).await {
                error!("ERROR: {}", error);
            }
        });

        Ok(receiver)
    }

    async fn check_files(&mut self, file_paths: Vec<impl AsRef<Path>>, rom_mode: RomsetMode) -> Result<Report> {
        let mut rx = self.send_sets_from_files(file_paths).await?;

        let mut r = vec![];
        while let Some(message) = rx.recv().await {
            let file_name = message.file_name;
            if !file_name.eq("") {
                if let Some(reporter) = self.reporter.as_mut() {
                    reporter.update_report_new_file(file_name.as_str());
                };
            }
            match message.content {
                ReportMessageContent::FoundGameSet(game_set) => {
                    if let Some(file_report) = self.build_file_report(file_name, game_set, rom_mode).await {
                        if let Some(reporter) = self.reporter.as_mut() {
                            reporter.update_report_new_added_file(1);
                        };
                        r.push(file_report)
                    } else {
                        if let Some(reporter) = self.reporter.as_mut() {
                            reporter.update_report_file_error(1);
                        };
                    }
                }
                ReportMessageContent::FoundNotValid => {
                    // warn!("File `{}` is not a valid file", file_name);
                    // TODO: Unknown file, fix. FileReport type wrong?
                    if let Some(reporter) = self.reporter.as_mut() {
                        reporter.update_report_ignored(1);
                    };
                }
                ReportMessageContent::FoundError => {
                    if let Some(reporter) = self.reporter.as_mut() {
                        reporter.update_report_file_error(1);
                    };
                },
                ReportMessageContent::Done => {
                    break;
                }
            }
        };

        let report = Report::from_files(rom_mode, r);

        if let Some(reporter) = self.reporter.as_mut() {
            reporter.finish();
        }
        Ok(report)
    }

    async fn build_file_report(&mut self, file_name: String, game_set: GameSet, rom_mode: RomsetMode) -> Option<FileReport> {
        let game_name = game_set.game.name.clone();
        let sets_and_unknowns_result = self.on_set_found(game_set, rom_mode);

        match sets_and_unknowns_result {
            Ok(sets_and_unknowns) => {
                let mut file_report = FileReport::new(file_name, rom_mode);
                file_report.sets = sets_and_unknowns.0;
                file_report.unknown = sets_and_unknowns.1;
                Some(file_report)
            }
            Err(e) => { 
                error!("Error getting report for game set `{}`: {}", game_name, e);
                None
            }
        }
    }

    fn on_set_found(&mut self, game_set: GameSet, rom_mode: RomsetMode) -> Result<(Vec<SetReport>, Vec<DataFile>)> {
        let mut set_reports = vec![];
        let mut unknowns= vec![];

        let rom_usage_result = self.data_reader.get_romsets_from_roms(game_set.roms, rom_mode)?;

        for entry in rom_usage_result.set_results {
            let set_name = entry.0;
            let roms = entry.1;

            let set_report = self.compare_roms_with_set(roms.into_iter().collect(), set_name, rom_mode)?;

            set_reports.push(set_report);
        };

        for unknown in rom_usage_result.unknowns {
            unknowns.push(unknown);
        }

        Ok((set_reports, unknowns))
    }

    fn compare_roms_with_set(&mut self, roms: Vec<DataFile>, set_name: String, rom_mode: RomsetMode) -> Result<SetReport> {
        let mut db_roms = self.data_reader.get_romset_roms(&set_name, rom_mode)?;

        let mut report = SetReport::new(set_name.clone());

        roms.into_iter().for_each(|rom| {
            let found_rom = db_roms.iter().position(|set_rom| {
                rom.deep_compare(&set_rom, FileChecks::ALL, false).ok().unwrap_or_else(|| false)
            });

            match found_rom {
                Some(set_rom_pos) => {
                    let set_rom = db_roms.remove(set_rom_pos);
                    if rom.name == set_rom.name {
                        report.roms_have.push(set_rom);
                    } else {
                        let file_rename = FileRename::new(rom, set_rom.name);
                        report.roms_to_rename.push(file_rename);
                    }
                }
                None => {
                    warn!("Rom `{}` couldn't be matched for set `{}`", rom, set_name);
                }
            }
        });

        report.roms_missing = db_roms.into_iter().filter_map(|rom| {
            Some(rom)
        }).collect();

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::Path, rc::Rc};
    use rusqlite::{Connection, OpenFlags};
    use crate::data::{importer::DatImporter, reader::sqlite::DBReader, writer::sqlite::DBWriter};
    use super::*;

    fn get_db_connection<'a, 'b>(dat_path: &'b impl AsRef<Path>) -> Result<Connection> {
        let mut conn = Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
        let writer = DBWriter::from_connection(&mut conn, 100);
        let mut importer = DatImporter::from_path(dat_path, writer)?;
        match importer.load_dat() {
            Ok(_) => {}
            Err(e) => { println!("ERROR {:?}", e);}
        }

        Ok(conn)
    }

    struct TestReportReporter {
        inner: Rc<RefCell<InnerReportReporter>>
    }

    struct InnerReportReporter {
        total_files: usize,
        current_files: usize,
        new_files: usize,
        directories: usize,
        ignored: usize,
        error: usize,
        finished: bool,
        files: Vec<String>
    }

    impl TestReportReporter {
        fn new() -> Self { Self { 
            inner: Rc::new(RefCell::new(InnerReportReporter {
                total_files: 0, current_files: 0, new_files: 0, directories: 0, ignored: 0, error: 0, finished: false, files: vec![] }
             )) }
        }
    }

    impl ReportReporter for TestReportReporter {
        fn set_total_files(&mut self, total_files: usize) {
            self.inner.borrow_mut().total_files += total_files;
        }

        fn update_report_new_file(&mut self, new_file: &str) {
            self.inner.borrow_mut().current_files += 1;
            self.inner.borrow_mut().files.push(new_file.to_string());
        }

        fn update_report_new_added_file(&mut self, new_files: usize) {
            self.inner.borrow_mut().new_files += new_files;
        }

        fn update_report_directory(&mut self, new_files: usize) {
            self.inner.borrow_mut().directories += new_files;
        }

        fn update_report_ignored(&mut self, new_files: usize) {
            self.inner.borrow_mut().ignored += new_files;
        }

        fn update_report_file_error(&mut self, new_files: usize) {
            self.inner.borrow_mut().error += new_files;
        }

        fn finish(&mut self) {
            self.inner.borrow_mut().finished = true;
        }
    }

    fn assert_file_report(report: &Report, file_name: &str, report_name: &str, roms_have: usize, roms_missing: usize, roms_to_rename: usize, roms_unneeded: usize) {
        let report_sets = &report.files;
        let assert_result = report_sets.iter().filter(|file_report| {
            file_report.file_name == file_name &&
            file_report.unknown.len() == roms_unneeded &&
            if file_report.sets.len() == 1 {
                let set_report = &file_report.sets[0];
                set_report.name == report_name
                && set_report.roms_have.len() == roms_have
                && set_report.roms_missing.len() == roms_missing
                && set_report.roms_to_rename.len() == roms_to_rename
            } else {
                false
            }
        }).collect::<Vec<_>>().len();
        if assert_result != 1{
            println!("Test failed with asserting filename {}, found {} coincidences.\nReport:\n{}",
                file_name,
                assert_result,
                report);
        }
        assert_eq!(assert_result, 1);
    }

    #[tokio::test]
    async fn get_right_data_from_file() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut reporter = Reporter::new(data_reader);
        let report_reporter = TestReportReporter::new();
        let inner = Rc::clone(&report_reporter.inner);

        reporter.add_reporter(report_reporter);

        let game_path = Path::new("testdata").join("split");
        let report = reporter.check(vec![ game_path ], RomsetMode::Merged).await?;

        assert_eq!(inner.borrow().total_files, 5);
        assert_eq!(inner.borrow().current_files, 5);
        assert_eq!(inner.borrow().new_files, 5);
        assert_eq!(inner.borrow().directories, 0);
        assert_eq!(inner.borrow().ignored, 0);
        assert_eq!(inner.borrow().error, 0);
        assert!(inner.borrow().finished);
        let report_sets = &report.files;
        assert_eq!(report_sets.len(), 5);
        tests::assert_file_report(&report, "device1.zip", "device1", 1, 0, 0, 0);
        tests::assert_file_report(&report, "game1.zip", "game1", 4, 2, 0, 0);
        tests::assert_file_report(&report, "game1a.zip", "game1", 2, 4, 0, 0);
        tests::assert_file_report(&report, "game2.zip", "game2", 3, 0, 0, 0);
        tests::assert_file_report(&report, "game3.zip", "game3", 3, 0, 0, 0);

        Ok(())
    }

    #[tokio::test]
    async fn get_wrong_data_from_file() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut reporter = Reporter::new(data_reader);
        let report_reporter = TestReportReporter::new();
        let inner = Rc::clone(&report_reporter.inner);

        reporter.add_reporter(report_reporter);

        let game_path = Path::new("testdata").join("wrong");
        let report = reporter.check(vec![ &game_path ], RomsetMode::Split).await?;

        assert_eq!(inner.borrow().total_files, 4);
        assert_eq!(inner.borrow().current_files, 4);
        assert_eq!(inner.borrow().new_files, 3);
        assert_eq!(inner.borrow().directories, 0);
        assert_eq!(inner.borrow().ignored, 1);
        assert_eq!(inner.borrow().error, 0);
        assert!(inner.borrow().finished);
        let report_sets = &report.files;
        assert_eq!(report_sets.len(), 3);
        tests::assert_file_report(&report, "game1.zip", "game1", 3, 1, 0, 0);
        tests::assert_file_report(&report, "game2.zip", "game2", 2, 0, 1, 0);
        tests::assert_file_report(&report, "game3.zip", "game3", 3, 0, 0, 1);

        Ok(())
    }
}