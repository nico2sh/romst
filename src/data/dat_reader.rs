
use std::{fs::File, fs, io::{BufRead, BufReader}, path::Path, sync::mpsc, str, thread};
use log::{debug, error, info, warn};
use anyhow::Result;
use quick_xml::{Reader, events::{attributes::Attributes, Event}};
use crate::{reporter::DatReaderReporter, data::writer::*, err, error::RomstError};

use super::models::{game::Game, file::DataFile, file::FileType};

pub struct DatReader<T: BufRead, W: DataWriter> {
    reader: Reader<T>,
    writer: W,
    reporter: DatReaderReporter,
}

impl<T: BufRead, W: DataWriter> DatReader<T, W> {
    pub fn from_path(path: &Path, writer: W) -> DatReader<BufReader<File>, W> {
        let (sender, receiver) = mpsc::channel::<u64>();

        let file_size = fs::metadata(path).unwrap().len();
        let reporter = DatReaderReporter::new(file_size);
        DatReader {
            reader: Reader::from_file(path).unwrap(),
            writer,
            reporter,
        }
    }

    pub fn from_string(xml: &str, writer: W) -> DatReader<&[u8], W> {
        let file_size = xml.len();
        let reporter = DatReaderReporter::new(file_size as u64);

        DatReader {
            reader: Reader::from_str(xml),
            writer,
            reporter,
        }
    }

    fn reader(&self) -> &Reader<T> {
        &self.reader
    }

    fn reader_mut(&mut self) -> &mut Reader<T> {
        &mut self.reader
    }

    fn reporter(&self) -> &DatReaderReporter {
        &self.reporter
    }

    fn repot_new_entry(&mut self) {
        let buf_pos = self.buf_pos() as u64;
        self.reporter.update_position(buf_pos);
    }

    fn buf_pos(&self) -> usize {
        self.reader().buffer_position()
    }

    pub fn load_dat(&mut self) -> Result<()> {
        self.reader_mut().trim_text(true);

        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf)? {
                Event::Start(ref e) if (e.name() == b"datafile") => {
                    self.read_datafile()?;
                }
                Event::Eof => {
                    self.reporter().finish();
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
            match self.reader_mut().read_event(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name() {
                        b"machine" | b"game" => self.read_rom_entry( String::from_utf8(e.name().to_vec())?, e.attributes())?,
                        b"header" => self.read_header()?,
                        tag_name => self.consume_tag(String::from_utf8(tag_name.to_vec())?)?,
                    }
                },
                Event::End(e) => {
                    if e.name() == b"datafile" {
                        return Ok(());
                    } else {
                        return err!(RomstError::UnexpectedTagClose { 
                            expected: String::from("datafile"),
                            found: String::from_utf8(e.name().to_vec())?,
                            position: self.buf_pos() });
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
        loop {
            match self.reader_mut().read_event(&mut buf)? {
                Event::Start(ref e) => {
                    self.consume_tag(String::from_utf8(e.name().to_vec())?)?;
                },
                Event::End(e) => {
                    if e.name() == tag_name.as_bytes() {
                        return Ok(());
                    } else {
                        return err!(RomstError::UnexpectedTagClose { 
                            expected: tag_name,
                            found: String::from_utf8(e.name().to_vec())?,
                            position: self.buf_pos() });
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
        match self.reader_mut().read_event(&mut buf)? {
            Event::Text(t) => {
                text = t.unescape_and_decode(self.reader())?
            },
            Event::End(_e) => {
                return Ok(String::from(""));
            },
            Event::Eof => return err!(RomstError::UnexpectedEOF),
            _e => return err!(RomstError::UnexpectedXMLTag {
                position: self.reader().buffer_position()
            }),
        }

        match self.reader_mut().read_event(&mut buf)? {
            Event::End(_e) => {
                return Ok(text);
            },
            Event::Eof => return err!(RomstError::UnexpectedEOF),
            _e => return err!(RomstError::UnexpectedXMLTag { position: self.reader().buffer_position() }),
        }
    }

    fn read_header(&mut self) -> Result<()> {
        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name() {
                        b"name" => {
                            let name = self.get_text()?;
                            info!("Name: {}", name);
                        },
                        b"description" => {
                            let desc = self.get_text()?;
                            info!("Description: {}", desc);
                        },
                        b"category" => {
                            let cat = self.get_text()?;
                            info!("Category: {}", cat);
                        },
                        b"version" => {
                            let ver = self.get_text()?;
                            info!("Version: {}", ver);
                        },
                        b"comment" => {
                            let comment = self.get_text()?;
                            info!("Comment: {}", comment);
                        },
                        tag_name => {
                            // we consume the tag
                            self.consume_tag(String::from_utf8(tag_name.to_vec())?)?;
                        },
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

    fn read_rom_entry(&mut self, entry_type: String, attributes: Attributes) -> Result<()> {
        let game = game_from_attributes(attributes)?;
        self.writer.on_new_game(&game)?;

        let mut roms = vec![];
        let mut samples = vec![];
        let mut disks = vec![];

        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name() {
                        b"description" => {
                            let desc = self.get_text()?;
                        },
                        b"year" => {
                            let year = self.get_text()?;
                        },
                        b"manufacturer" => {
                            let manuf = self.get_text()?;
                        },
                        n => self.consume_tag(String::from_utf8(n.to_vec())?)?
                    }
                },
                Event::Empty(e) => {
                    match e.name() {
                        b"rom" => {
                            let rom = file_from_attributes(FileType::Rom, e.attributes())?;
                            roms.push(rom);
                        },
                        b"sample" => {
                            let sample = file_from_attributes(FileType::Sample, e.attributes());
                            samples.push(sample);
                        },
                        b"disk" => {
                            let disk = file_from_attributes(FileType::Disk, e.attributes());
                            disks.push(disk);
                        },
                        _ => ()
                    }
                },
                Event::End(e) => {
                    if String::from_utf8(e.name().to_vec())? == entry_type {
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

        self.writer.on_new_roms(&game, &roms)?;
        self.repot_new_entry();

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
                    (Ok(k), Ok(v)) => f(k, v),
                    (Err(e), Ok(_)) | (Ok(_), Err(e)) => error!("Error reading attributes: {}", e),
                    (Err(e1), Err(e2)) => error!("Error reading attributes: {}, {}", e1, e2),
                }
            },
            Err(e) => {
                error!("Error reading attributes: {}", e);
                ()
            }
        }
    });
}

fn file_from_attributes(file_type: FileType, attributes: Attributes) -> Result<DataFile> {
    let mut data_file = DataFile::new(file_type);

    process_attributes(attributes, |key, value| {
        match key {
            "name" => data_file.name = Some(String::from(value)),
            "sha1" => data_file.sha1 = Some(String::from(value)),
            "md5" => data_file.md5 = Some(String::from(value)),
            "crc" => data_file.crc = Some(String::from(value)),
            "size" => data_file.size = value.parse::<u32>().ok(),
            "serial" => debug!("Ignoring serial attribute from file"),
            "status" => data_file.status = Some(String::from(value)),
            k => debug!("Unknown atribute parsing: {}", k),
        }
    });

    Ok(data_file)
}

fn game_from_attributes(attributes: Attributes) -> Result<Game> {
    let mut game = Game::new(String::from(""));

    process_attributes(attributes, |key, value| {
        match key {
            "name" => game.name = String::from(value),
            "cloneof" => game.clone_of = Some(String::from(value)),
            "romof" => game.rom_of = Some(String::from(value)),
            "sourcefile" => game.source_file = Some(String::from(value)),
            k => debug!("Unknown atribute parsing: {}", k),
        }
    });

    if game.name == "" {
        return err!(RomstError::ParsingError { message: String::from("Missing name attribute for Game") });
    }

    Ok(game)
}