
use std::{fs::{self, File}, io::{BufRead, BufReader}, path::Path, str};
use log::{debug, error, info};
use anyhow::Result;
use quick_xml::{Reader, events::{attributes::Attributes, Event}};
use crate::{data::writer::*, err, error::RomstError};

use super::models::{file::DataFile, file::{DataFileInfo, FileType}, game::Game};

pub struct DatImporter<R: BufRead, W: DataWriter> {
    reader: Reader<R>,
    writer: W,
    reporter: Option<Box<dyn DatImporterReporter>>,
    total_bytes: u64,
}

pub trait DatImporterReporter {
    fn set_total_bytes(&mut self, total_bytes: u64);
    fn update_position(&mut self, bytes: u64, new_entries: u32);
    fn start_finish(&self);
    fn finish(&self);
}

impl<W: DataWriter> DatImporter<BufReader<File>, W> {
    pub fn from_path(path: &impl AsRef<Path>, writer: W) ->Result<Self> {
        let total_bytes = fs::metadata(path)?.len();
        Ok(DatImporter {
            reader: Reader::from_file(path).unwrap(),
            writer,
            reporter: None,
            total_bytes
        })
    }
}

impl<R: BufRead, W: DataWriter> DatImporter<R, W> {
    pub fn set_reporter<P>(&mut self, mut reporter: P) where P: DatImporterReporter + 'static {
        reporter.set_total_bytes(self.total_bytes);
        self.reporter = Some(Box::new(reporter));
    }

    fn report_new_entry(&mut self, new_entries: u32) {
        let buf_pos = self.buf_pos() as u64;
        if let Some(reporter) = self.reporter.as_mut() {
            reporter.update_position(buf_pos, new_entries);
        }
    }

    fn buf_pos(&self) -> usize {
        self.reader.buffer_position()
    }

    pub fn load_dat(&mut self) -> Result<()> {
        self.reader.trim_text(true);

        self.writer.init()?;

        let mut buf = Vec::new();
        loop {
            match self.reader.read_event(&mut buf)? {
                Event::Start(ref e) => {
                    if let Ok(name) = str::from_utf8(e.name()) {
                        match name.to_lowercase().trim() {
                            "datafile" => {
                                self.read_datafile()?;
                            },
                            "mame" => {
                                self.read_mame_header(e.attributes());
                                self.read_datafile()?;
                            },
                            _ => {} 
                        }
                    }
                },
                Event::Eof => {
                    if let Some(ref reporter) = self.reporter {
                        reporter.start_finish();
                    }
                    self.writer.finish()?;
                    if let Some(ref reporter) = self.reporter {
                        reporter.finish();
                    }
                    break
                }, 
                _ => (),
            }
            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        }

        Ok(())
    }

    fn read_datafile(&mut self) -> Result<()> {
        let mut buf = Vec::new();
        loop {
            match self.reader.read_event(&mut buf)? {
                Event::Start(ref e) => {
                    if let Ok(name) = str::from_utf8(e.name()) {
                        match name.to_lowercase().trim() {
                            "machine" | "game" => self.read_game_entry( String::from_utf8(e.name().to_vec())?, e.attributes())?,
                            "header" => self.read_dat_header()?,
                            tag_name => self.consume_tag(tag_name.to_string())?,
                        }
                    }
                },
                Event::End(e) => {
                    if let Ok(name) = str::from_utf8(e.name()){
                        if name.to_lowercase().trim() == "datafile" {
                            return Ok(());
                        } else {
                            return err!(RomstError::UnexpectedTagClose { 
                                expected: String::from("datafile"),
                                found: String::from_utf8(e.name().to_vec())?,
                                position: self.buf_pos() });
                        }
                    }
                },
                Event::Eof => return err!(RomstError::UnexpectedEOF),
                _ => (),
            }
            buf.clear();
        }
    }

    // Reads through a tag moving the reader until it closes the tag
    fn consume_tag(&mut self, tag_name: String) -> Result<()> {
        let mut buf = Vec::new();
        let tag = tag_name.to_lowercase();
        loop {
            match self.reader.read_event(&mut buf)? {
                Event::Start(ref e) => {
                    self.consume_tag(str::from_utf8(e.name())?.trim().to_string())?;
                },
                Event::End(e) => {
                    if let Ok(name) = str::from_utf8(e.name()) {
                        if name.trim().to_lowercase() == tag {
                            return Ok(());
                        } else {
                            return err!(RomstError::UnexpectedTagClose { 
                                expected: tag,
                                found: String::from_utf8(e.name().to_vec())?,
                                position: self.buf_pos() });
                        }
                    }
                },
                Event::Eof => return err!(RomstError::UnexpectedEOF),
                _ => (),
            }
            buf.clear();
        }
    }

    fn get_text(&mut self) -> Result<String> {
        let mut buf = Vec::new();
        let text;
        match self.reader.read_event(&mut buf)? {
            Event::Text(t) => {
                text = t.unescape_and_decode(&self.reader)?.trim().to_string()
            },
            Event::End(_e) => {
                return Ok(String::from(""));
            },
            Event::Eof => return err!(RomstError::UnexpectedEOF),
            _e => return err!(RomstError::UnexpectedXMLTag {
                position: self.reader.buffer_position()
            }),
        }

        match self.reader.read_event(&mut buf)? {
            Event::End(_e) => {
                return Ok(text);
            },
            Event::Eof => return err!(RomstError::UnexpectedEOF),
            _e => return err!(RomstError::UnexpectedXMLTag { position: self.reader.buffer_position() }),
        }
    }

    fn read_mame_header(&mut self, attributes: Attributes) {
        process_attributes(attributes, |key, value| {
            let val = value.trim();
            match key.to_lowercase().trim() {
                "build" => {
                    let build = String::from(val);
                    info!("Build: {}", build);
                },
                "debug" => {
                    let debug = String::from(val);
                    info!("Debug: {}", debug);
                },
                "mameconfig" => {
                    let mameconfig = String::from(val);
                    info!("Mameconfig: {}", mameconfig);
                },
                k => debug!("Unknown atribute parsing: {}", k),
            }
        });
    }

    fn read_dat_header(&mut self) -> Result<()> {
        let mut buf = Vec::new();
        loop {
            match self.reader.read_event(&mut buf)? {
                Event::Start(ref e) => {
                    if let Ok(name) = str::from_utf8(e.name()) {
                        match name.to_lowercase().trim() {
                            "name" => {
                                let name = self.get_text()?;
                                info!("Name: {}", name);
                            },
                            "description" => {
                                let desc = self.get_text()?;
                                info!("Description: {}", desc);
                            },
                            "category" => {
                                let cat = self.get_text()?;
                                info!("Category: {}", cat);
                            },
                            "version" => {
                                let ver = self.get_text()?;
                                info!("Version: {}", ver);
                            },
                            "comment" => {
                                let comment = self.get_text()?;
                                info!("Comment: {}", comment);
                            },
                            tag_name => {
                                // we consume the tag
                                self.consume_tag(tag_name.trim().to_string())?;
                            },
                        }
                    }
                },
                Event::End(_) => break,
                Event::Eof => panic!("Unexpected end of file"),
                _ => (),
            }
        }
        buf.clear();
        Ok(())
    }

    fn read_game_entry(&mut self, entry_type: String, attributes: Attributes) -> Result<()> {
        let mut game = game_from_attributes(attributes)?;

        let mut roms = vec![];
        let mut samples = vec![];
        let mut disks = vec![];
        let mut devices = vec![];

        let mut buf = Vec::new();
        loop {
            match self.reader.read_event(&mut buf)? {
                Event::Start(ref e) => {
                    if let Ok(name) = str::from_utf8(e.name()) {
                        match name.to_lowercase().trim() {
                            "description" => {
                                let desc = self.get_text()?;
                                game.info_description = Some(desc);
                            },
                            "year" => {
                                let year = self.get_text()?;
                                game.info_year = Some(year);
                            },
                            "manufacturer" => {
                                let manuf = self.get_text()?;
                                game.info_manufacturer = Some(manuf);
                            },
                            n => self.consume_tag(n.trim().to_string())?
                        }
                    }
                },
                Event::Empty(e) => {
                    if let Ok(name) = str::from_utf8(e.name()) {
                        match name.to_lowercase().trim() {
                            "rom" => {
                                let rom = file_from_attributes(FileType::Rom, e.attributes())?;
                                roms.push(rom);
                            },
                            "sample" => {
                                let sample = device_ref(e.attributes());
                                if let Some(sample_name) = sample {
                                    samples.push(sample_name);
                                }
                            },
                            "disk" => {
                                let disk = file_from_attributes(FileType::Disk, e.attributes())?;
                                disks.push(disk);
                            },
                            "device_ref" => {
                                let device = device_ref(e.attributes());
                                if let Some(device_name) = device {
                                    devices.push(device_name);
                                }
                            },
                            _ => ()
                        }
                    }
                },
                Event::End(e) => {
                    if str::from_utf8(e.name())?.trim() == entry_type {
                        break;
                    } else {
                        return err!(RomstError::UnexpectedTagClose { 
                            expected: entry_type,
                            found: String::from_utf8(e.name().to_vec())?,
                            position: self.buf_pos() });
                    }
                },
                _ => ()
            }
            buf.clear();
        }

        self.writer.on_new_entry(game, roms, disks, samples, devices)?;
        self.report_new_entry(1);

        Ok(())
    }
}

// Helper functions
fn process_attributes<F>(attributes: Attributes, mut f: F) where F: FnMut(&str, &str) {
    attributes.for_each(|a| {
        match a {
            Ok(a) => {
                let key = str::from_utf8(a.key);
                let value = str::from_utf8(&a.value);

                match (key, value) {
                    (Ok(k), Ok(v)) => f(k.trim(), v.trim()),
                    (Err(e), Ok(_)) | (Ok(_), Err(e)) => error!("Error reading attributes: {}", e),
                    (Err(e1), Err(e2)) => error!("Error reading attributes: {}, {}", e1, e2),
                }
            },
            Err(e) => {
                error!("Error reading attributes: {}", e)
            }
        }
    });
}

fn device_ref(attributes: Attributes) -> Option<String> {
    let mut device_name = None;
    process_attributes(attributes, |key, value| {
        match key.to_lowercase().as_str() {
            "name" => device_name = Some(String::from(value)),
            k => debug!("Unknown atribute parsing: {}", k),
        }
    });

    device_name
}

fn file_from_attributes(file_type: FileType, attributes: Attributes) -> Result<DataFile> {
    let mut data_file_info = DataFileInfo::new(file_type);
    let mut file_name = None;
    let mut status = None;

    process_attributes(attributes, |key, value| {
        match key.to_lowercase().as_str() {
            "name" => file_name = Some(value.to_string()),
            "sha1" => data_file_info.sha1 = Some(String::from(value)),
            "md5" => data_file_info.md5 = Some(String::from(value)),
            "crc" => data_file_info.crc = Some(String::from(value).to_lowercase()),
            "size" => data_file_info.size = value.parse::<u32>().ok(),
            "serial" => debug!("Ignoring serial attribute from file"),
            "status" => status = Some(String::from(value)),
            k => debug!("Unknown atribute parsing: {}", k),
        }
    });

    if let Some(name) = file_name {
        Ok(DataFile::new_with_status(name, data_file_info, status))
    } else {
        error!("Found file without name, not adding");
        err!(RomstError::ParsingError { message: "File without name".to_string() })
    }
}

fn game_from_attributes(attributes: Attributes) -> Result<Game> {
    let mut game = Game::new(String::from(""));

    process_attributes(attributes, |key, value| {
        match key.to_lowercase().as_str() {
            "name" => game.name = String::from(value),
            "cloneof" => game.clone_of = Some(String::from(value)),
            "romof" => game.rom_of = Some(String::from(value)),
            "sourcefile" => game.source_file = Some(String::from(value)),
            "sampleof" => game.sample_of = Some(String::from(value)),
            k => debug!("Unknown atribute parsing: {}", k),
        }
    });

    if game.name == "" {
        return err!(RomstError::ParsingError { message: String::from("Missing name attribute for Game") });
    }

    Ok(game)
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};

    use super::*;

    pub struct MemoryWriter {
        pub initialized: Rc<RefCell<bool>>,
        pub games: Rc<RefCell<Vec<String>>>,
    }

    impl MemoryWriter {
        pub fn new() -> Self {
            MemoryWriter {
                initialized: Rc::new(RefCell::new(false)),
                games: Rc::new(RefCell::new(vec![])),
            }
        }
    }

    impl DataWriter for MemoryWriter {
        fn init(&self) -> Result<()> {
            self.initialized.replace(true);
            Ok(())
        }

        fn finish(&mut self) -> Result<()> {
            Ok(())
        }

        fn on_new_entry(&mut self, game: Game, roms: Vec<DataFile>, disks: Vec<DataFile>, samples: Vec<String>, device_refs: Vec<String>) -> Result<()> {
            self.games.borrow_mut().push(game.name);

            Ok(())
        }
    }

    #[test]
    fn read_xml() -> Result<()> {
        let writer = MemoryWriter::new();
        let games = Rc::clone(&writer.games);
        let initialized = Rc::clone(&writer.initialized);

        let path = Path::new("testdata").join("test.dat");
        let mut importer =
            DatImporter::from_path(&path, writer)?;
        importer.load_dat()?;
        
        assert!(*initialized.borrow());

        println!("{:?}", *games.borrow());
        
        Ok(())
    }
}