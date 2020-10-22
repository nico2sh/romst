use std::{borrow::Cow, error::Error, fs::File, io::BufRead, io::BufReader, collections::HashSet, option::Option, path::Path, str};
use quick_xml::{Reader, events::{attributes::Attributes, BytesStart}};
use quick_xml::events::Event;

pub struct DatReader<T: BufRead> {
    reader: Reader<T>,
}

impl<T: BufRead> DatReader<T> {
    pub fn from_path(path: &Path) -> DatReader<BufReader<File>> {
        DatReader {
            reader: Reader::from_file(path).unwrap(),
        }
    }

    pub fn from_string(xml: &str) -> DatReader<&[u8]> {
        DatReader {
            reader: Reader::from_str(xml),
        }
    }

    fn reader(&self) -> &Reader<T> {
        &self.reader
    }

    fn reader_mut(&mut self) -> &mut Reader<T> {
        &mut self.reader
    }

    pub fn load_dat(&mut self) -> Result<(), Box<dyn Error>> {
        self.reader_mut().trim_text(true);

        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf) {
                Ok(Event::Start(ref e)) if (e.name() == b"datafile") => {
                    self.read_datafile();
                }
                Ok(Event::Eof) => {
                    println!("EOF");
                    break
                }, 
                Err(e) => panic!("Error at position {}: {:?}", self.reader().buffer_position(), e),
                _ => (),
            }

            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        }

        //println!("Text: {:?}", txt);
        Ok(())
    }

    fn get_next_event(&mut self, buf: &'static mut Vec<u8>) -> Event {
        let reader = self.reader_mut();
        let res = reader.read_event(buf);
        match res {
            Ok(event) => {
                return event
            },
            Err(e) => panic!("Error reading datafile at position {}: {:?}", self.reader().buffer_position(), e),
        }
    }

    fn panic_string(&self) -> String {
        format!("Error reading datafile at position {}", self.reader().buffer_position())
    }

    fn read_datafile(&mut self) {
        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf).expect(self.panic_string().as_str()) {
                Event::Start(ref e) => {
                    match e.name() {
                        b"header" => self.read_header(),
                        b"machine" => self.read_machine(e.attributes()),
                        _ => ()
                    }
                },
                // Event::End(_) => break,
                Event::Eof => panic!("Unexpected end of file"),
                _ => (),
            }
            buf.clear();
        }
    }

    fn read_header(&mut self) {
        let mut buf = Vec::new();
        loop {
            match self.reader_mut().read_event(&mut buf).expect(self.panic_string().as_str()) {
                Event::Start(ref e) if (e.name() == b"name") => {
                    self.read_header_name();
                },
                Event::End(_) => break,
                Event::Eof => panic!("Unexpected end of file"),
                _ => (),
            }
        }
        buf.clear();
    }

    fn read_header_name(&mut self) {
        let mut buf = Vec::new();
        match self.reader_mut().read_event(&mut buf).expect(self.panic_string().as_str()) {
            Event::Text(ref e) => {
                let unescaped = e.unescape_and_decode(self.reader());
                match unescaped {
                    Ok(name) => { 
                        println!("Name: {}", name);
                    },
                    Err(_) => ()
                }
            },
            Event::Eof => panic!("Unexpected end of file"),
            _ => (),
        }
        buf.clear();
    }

    fn read_machine(&mut self, attributes: Attributes) {
        println!("Machine reading");
        let mut keys = HashSet::new();
        self.process_attributes(attributes, |key, value| {
            keys.insert(format!("{}: {}", key, value));
            //println!("{}: {}", key, value);
        });

        println!("keys: {:?}", keys);
    }

    fn process_attributes<F>(&mut self, attributes: Attributes, mut f: F) where F: FnMut(String, String) {
        attributes.for_each(|a| {
            match a {
                Ok(a) => {
                    let key = String::from_utf8(a.key.to_vec());
                    let value = String::from_utf8(a.value.to_vec());

                    match (key, value) {
                        (Ok(k), Ok(v)) => f(k, v),
                        (Err(e), Ok(_)) => println!("Error reading attributes: {}", e),
                        (Ok(_), Err(e)) => println!("Error reading attributes: {}", e),
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
