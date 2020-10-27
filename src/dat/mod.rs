use std::{fs::File, io::BufRead, io::BufReader, collections::HashSet, path::Path, str};
use anyhow::Result;
use anyhow::anyhow;
use quick_xml::{Reader, events::attributes::Attributes};
use quick_xml::events::Event;
use crate::error::RomstError;

pub struct DatReader<T: BufRead> {
    reader: Reader<T>,
    buf: Vec<u8>
}

impl<T: BufRead> DatReader<T> {
    pub fn from_path(path: &Path) -> DatReader<BufReader<File>> {
        DatReader {
            reader: Reader::from_file(path).unwrap(),
            buf: Vec::new(),
        }
    }

    pub fn from_string(xml: &str) -> DatReader<&[u8]> {
        DatReader {
            reader: Reader::from_str(xml),
            buf: Vec::new(),
        }
    }

    fn reader(&self) -> &Reader<T> {
        &self.reader
    }

    fn reader_mut(&mut self) -> &mut Reader<T> {
        &mut self.reader
    }

    fn buf_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buf
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
                    println!("EOF");
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
                        return Err(anyhow!(RomstError::UnexpectedTagClose { 
                            expected: String::from("datafile"),
                            found: String::from_utf8(e.name().to_vec())?,
                            position: self.buf_pos() }));
                    }
                },
                Event::Eof => return Err(anyhow!(RomstError::UnexpectedEOF)),
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
                        return Err(anyhow!(RomstError::UnexpectedTagClose { 
                            expected: tag_name,
                            found: String::from_utf8(e.name().to_vec())?,
                            position: self.buf_pos() }));
                    }
                },
                Event::Eof => return Err(anyhow!(RomstError::UnexpectedEOF)),
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
            Event::Eof => return Err(anyhow!(RomstError::UnexpectedEOF)),
            _e => return Err(anyhow!(RomstError::UnexpectedXMLTag {
                position: self.reader().buffer_position()
            })),
        }

        match self.reader_mut().read_event(&mut buf)? {
            Event::End(_e) => {
                return Ok(text);
            },
            Event::Eof => return Err(anyhow!(RomstError::UnexpectedEOF)),
            _e => return Err(anyhow!(RomstError::UnexpectedXMLTag { position: self.reader().buffer_position() })),
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
                            println!("Name: {}", name);
                        },
                        b"description" => {
                            let desc = self.get_text()?;
                            println!("Description: {}", desc);
                        },
                        b"category" => {
                            let cat = self.get_text()?;
                            println!("Category: {}", cat);
                        },
                        b"version" => {
                            let ver = self.get_text()?;
                            println!("Version: {}", ver);
                        },
                        b"comment" => {
                            let comment = self.get_text()?;
                            println!("Comment: {}", comment);
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

    fn read_rom_entry(&mut self, entry_name: String, attributes: Attributes) -> Result<()> {
        let mut keys = HashSet::new();
        self.process_attributes(attributes, |key, value| {
            keys.insert(format!("{}: {}", key, value));
            //println!("{}: {}", key, value);
        });

        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf)? {
                Event::Start(ref e) => {
                    match e.name() {
                        b"description" => {
                            let desc = self.get_text()?;
                            println!("{}", desc);
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
                            self.process_attributes(e.attributes(), |key, value| {

                            });
                        },
                        b"sample" => {
                            self.process_attributes(e.attributes(), |key, value| {

                            });
                        },
                        b"disk" => {
                            self.process_attributes(e.attributes(), |key, value| {

                            });
                        },
                        _ => ()
                    }
                },
                Event::End(e) => {
                    if String::from_utf8(e.name().to_vec())? == entry_name {
                        break;
                    } else {
                        return Err(anyhow!(RomstError::UnexpectedTagClose { 
                            expected: entry_name,
                            found: String::from_utf8(e.name().to_vec())?,
                            position: self.buf_pos() }));
                    }
                },
                _ => ()
            }
            buf.clear();
        }
        println!("keys: {:?}", keys);
        Ok(())
    }

    fn process_attributes<F>(&mut self, attributes: Attributes, mut f: F) where F: FnMut(String, String) {
        attributes.for_each(|a| {
            match a {
                Ok(a) => {
                    let key = String::from_utf8(a.key.to_vec());
                    let value = String::from_utf8(a.value.to_vec());

                    match (key, value) {
                        (Ok(k), Ok(v)) => f(k, v),
                        (Err(e), Ok(_)) | (Ok(_), Err(e)) => println!("Error reading attributes: {}", e),
                        (Err(e1), Err(e2)) => println!("Error reading attributes: {}, {}", e1, e2),
                    }
                },
                Err(e) => {
                    println!("Error reading attributes: {}", e);
                    ()
                }
            }
        });
    }
}
