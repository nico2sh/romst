
use std::{fs::File, path::PathBuf, fs, io::{BufRead, BufReader}, path::Path, str};
use log::{debug, error, info};
use anyhow::Result;
use quick_xml::{Reader, events::{attributes::Attributes, Event}};
use crate::{sysout::DatImporterReporter, data::writer::*, err, error::RomstError};

use super::models::{game::Game, file::DataFile, file::FileType};

pub struct DatImporter<T: BufRead, W: DataWriter> {
    reader: Reader<T>,
    writer: W,
    reporter: DatImporterReporter,
}

impl<T: BufRead, W: DataWriter> DatImporter<T, W> {
    pub fn from_path(path: &impl AsRef<Path>, writer: W) -> DatImporter<BufReader<File>, W> {
        let file_size = fs::metadata(path).unwrap().len();
        let reporter = DatImporterReporter::new(file_size);
        DatImporter {
            reader: Reader::from_file(path).unwrap(),
            writer,
            reporter,
        }
    }

    pub fn from_string(xml: &str, writer: W) -> DatImporter<&[u8], W> {
        let file_size = xml.len();
        let reporter = DatImporterReporter::new(file_size as u64);

        DatImporter {
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

    fn repot_new_entry(&mut self, new_entries: u32) {
        let buf_pos = self.buf_pos() as u64;
        self.reporter.update_position(buf_pos, new_entries);
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
                },
                Event::Start(ref e) if (e.name() == b"mame") => {
                    self.read_mame_header(e.attributes());
                    self.read_datafile()?;
                },
                Event::Eof => {
                    self.reporter.start_finish();
                    self.writer.finish()?;
                    self.reporter.finish();
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
                        b"machine" | b"game" => self.read_game_entry( String::from_utf8(e.name().to_vec())?, e.attributes())?,
                        b"header" => self.read_dat_header()?,
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

    fn read_mame_header(&mut self, attributes: Attributes) {
        process_attributes(attributes, |key, value| {
            match key {
                "build" => {
                    let build = String::from(value);
                    info!("Build: {}", build);
                },
                "debug" => {
                    let debug = String::from(value);
                    info!("Debug: {}", debug);
                },
                "mameconfig" => {
                    let mameconfig = String::from(value);
                    info!("Mameconfig: {}", mameconfig);
                },
                k => debug!("Unknown atribute parsing: {}", k),
            }
        });
    }

    fn read_dat_header(&mut self) -> Result<()> {
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

    fn read_game_entry(&mut self, entry_type: String, attributes: Attributes) -> Result<()> {
        let mut game = game_from_attributes(attributes)?;

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
                            game.info_description = Some(desc);
                        },
                        b"year" => {
                            let year = self.get_text()?;
                            game.info_year = Some(year);
                        },
                        b"manufacturer" => {
                            let manuf = self.get_text()?;
                            game.info_manufacturer = Some(manuf);
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

        self.writer.on_new_game(game.clone())?;
        self.writer.on_new_roms(game, roms)?;
        self.repot_new_entry(1);

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
    let mut data_file = DataFile::new(file_type, "".to_string());

    process_attributes(attributes, |key, value| {
        match key {
            "name" => data_file.name = String::from(value),
            "sha1" => data_file.sha1 = Some(String::from(value)),
            "md5" => data_file.md5 = Some(String::from(value)),
            "crc" => data_file.crc = Some(String::from(value)),
            "size" => data_file.size = value.parse::<u32>().ok(),
            "serial" => debug!("Ignoring serial attribute from file"),
            "status" => data_file.status = Some(String::from(value)),
            k => debug!("Unknown atribute parsing: {}", k),
        }
    });

    if data_file.name.eq("") {
        error!("Found file without name, not adding");
        err!(RomstError::ParsingError { message: "File without name".to_string() })
    } else {
        Ok(data_file)
    }
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

#[cfg(test)]
mod tests {
    const XML: &str = r#"<datafile>
        <header>
        </header>
        <machine name="circus" sourcefile="circus.cpp">
            <description>Circus / Acrobat TV</description>
            <year>1977</year>
            <manufacturer>Exidy / Taito</manufacturer>
            <rom name="9000.1c" size="512" crc="1f954bb3" sha1="62a958b48078caa639b96f62a690583a1c8e83f5"/>
            <rom name="9001.2c" size="512" crc="361da7ee" sha1="6e6fe5b37ccb4c11aa4abbd9b7df772953abfe7e"/>
            <rom name="9002.3c" size="512" crc="30d72ef5" sha1="45fc8285e213bf3906a26205a8c0b22f311fd6c3"/>
            <rom name="9003.4c" size="512" crc="6efc315a" sha1="d5a4a64a901853fff56df3c65512afea8336aad2"/>
            <rom name="9004.1a" size="512" crc="7654ea75" sha1="fa29417618157002b8ecb21f4c15104c8145a742"/>
            <rom name="9005.2a" size="512" crc="b8acdbc5" sha1="634bb11089f7a57a316b6829954cc4da4523f267"/>
            <rom name="9006.3a" size="512" crc="901dfff6" sha1="c1f48845456e88d54981608afd00ddb92d97da99"/>
            <rom name="9007.5a" size="512" crc="9dfdae38" sha1="dc59a5f90a5a49fa071aada67eda768d3ecef010"/>
            <rom name="9008.6a" size="512" crc="c8681cf6" sha1="681cfea75bee8a86f9f4645e6c6b94b44762dae9"/>
            <rom name="9009.7a" size="512" crc="585f633e" sha1="46133409f42e8cbc095dde576ce07d97b235972d"/>
            <rom name="9010.8a" size="512" crc="69cc409f" sha1="b77289e62313e8535ce40686df7238aa9c0035bc"/>
            <rom name="9011.9a" size="512" crc="aff835eb" sha1="d6d95510d4a046f48358fef01103bcc760eb71ed"/>
            <rom name="9012.14d" size="512" crc="2fde3930" sha1="a21e2d342f16a39a07edf4bea8d698a52216ecba"/>
            <rom name="dm74s570-d4.4d" size="512" crc="aad8da33" sha1="1d60a6b75b94f5be5bad190ef56e9e3da20bf81a" status="baddump"/>
            <rom name="dm74s570-d5.5d" size="512" crc="ed2493fa" sha1="57ee357b68383b0880bfa385820605bede500747" status="baddump"/>
            <device_ref name="m6502"/>
            <device_ref name="screen"/>
            <device_ref name="gfxdecode"/>
            <device_ref name="palette"/>
            <device_ref name="speaker"/>
            <device_ref name="samples"/>
            <device_ref name="discrete"/>
            <sample name="bounce"/>
            <sample name="miss"/>
            <sample name="pop"/>
            <driver status="imperfect"/>
	    </machine>
        <machine name="robotbwl" sourcefile="circus.cpp">
            <description>Robot Bowl</description>
            <year>1977</year>
            <manufacturer>Exidy</manufacturer>
            <rom name="4010.4c" size="512" crc="a5f7acb9" sha1="556dd34d0fa50415b128477e208e96bf0c050c2c"/>
            <rom name="4011.3c" size="512" crc="d5380c9b" sha1="b9670e87011a1b3aebd1d386f1fe6a74f8c77be9"/>
            <rom name="4012.2c" size="512" crc="47b3e39c" sha1="393c680fba3bd384e2c773150c3bae4d735a91bf"/>
            <rom name="4013.1c" size="512" crc="b2991e7e" sha1="32b6d42bb9312d6cbe5b4113fcf2262bfeef3777"/>
            <rom name="4020.1a" size="512" crc="df387a0b" sha1="97291f1a93cbbff987b0fbc16c2e87ad0db96e12"/>
            <rom name="4021.2a" size="512" crc="c948274d" sha1="1bf8c6e994d601d4e6d30ca2a9da97e140ff5eee"/>
            <rom name="4022.3a" size="512" crc="8fdb3ec5" sha1="a9290edccb8f75e7ec91416d46617516260d5944"/>
            <rom name="4023.5a" size="512" crc="ba9a6929" sha1="9cc6e85431b5d82bf3a624f7b35ddec399ad6c80"/>
            <rom name="4024.6a" size="512" crc="16fd8480" sha1="935bb0c87d25086f326571c83f94f831b1a8b036"/>
            <rom name="4025.7a" size="512" crc="4cadbf06" sha1="380c10aa83929bfbfd89facb252e68c307545755"/>
            <rom name="4026a.8a" size="512" crc="bc809ed3" sha1="2bb4cdae8c9619eebea30cc323960a46a509bb58"/>
            <rom name="4027b.9a" size="512" crc="07487e27" sha1="b5528fb3fec474df2b66f36e28df13a7e81f9ce3"/>
            <rom name="5000.4d" size="32" status="nodump"/>
            <rom name="5001.5d" size="32" status="nodump"/>
            <rom name="6000.14d" size="32" crc="a402ac06" sha1="3bd75630786bcc86d9e9fbc826adc909eef9b41f"/>
            <device_ref name="m6502"/>
            <device_ref name="screen"/>
            <device_ref name="gfxdecode"/>
            <device_ref name="palette"/>
            <device_ref name="speaker"/>
            <device_ref name="samples"/>
            <device_ref name="discrete"/>
            <sample name="balldrop"/>
            <sample name="demerit"/>
            <sample name="hit"/>
            <sample name="reward"/>
            <sample name="roll"/>
            <driver status="imperfect"/>
        </machine>
        </datafile>
        "#;

    use rusqlite::Connection;

    use crate::data::writer::{memory::MemoryWriter, sysout::SysOutWriter};

    use super::*;

    #[test]
    fn read_xml() -> Result<()> {
        let db = Connection::open_in_memory()?;

        let writer = MemoryWriter::new();
        writer.init()?;

        let mut dat_reader: DatImporter<&[u8], SysOutWriter> =
            DatImporter::<BufReader<File>, SysOutWriter>::from_string(XML, SysOutWriter::new());
        
        dat_reader.load_dat()?;

        assert!(true);
        
        Ok(())
    }
}