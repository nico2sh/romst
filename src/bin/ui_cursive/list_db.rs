use std::{fs, path::Path, thread};

use cursive::{Cursive, View, align::HAlign, traits::{Boxable, Nameable, Scrollable}, views::{Dialog, DummyView, LinearLayout, Panel, SelectView, TextView}};
use romst::Romst;
use anyhow::Result;

use super::list_sets::ListSets;

const BASE_PATH: &str = "db";

pub struct SelectDB {

}

impl SelectDB {
    pub fn new() -> Self {
        Self { }
    }

    pub fn load_view(&self) -> Result<impl View> {
        let mut select_db = SelectView::new()
            .h_align(HAlign::Left)
            .autojump()
            .on_select(on_select_db)
            .on_submit(on_choose_db);
        
        let db_list = self.get_db_list()?;

        select_db.add_all(db_list.clone());

        let db_details_content = if !db_list.is_empty() {
            select_db = select_db.selected(0);
            ""
        } else {
            select_db.disable();
            "No DB file found, make sure you have a DB in the `./db` directory next to the Romst executable."
        };
        let db_details = TextView::new(db_details_content)
            .h_align(HAlign::Center);

        let dialog = Dialog::around(LinearLayout::horizontal()
            .child(select_db.with_name("selection_list").scrollable())
            .child(DummyView)
            .child(Panel::new(db_details.with_name("db_details"))
            .full_width()))
            .title("Select DB file")
            .full_screen();

        Ok(dialog)
    }

    fn get_db_list(&self) -> Result<Vec<(String, String)>> {
        let db_path = Path::new(BASE_PATH);

        if db_path.is_file() {
            fs::remove_file(db_path)?;
        };

        if !db_path.exists() {
            fs::create_dir(db_path)?;
        };

        let files = db_path.read_dir()?.into_iter().filter_map(|file| {
            match file {
                Ok(f) => { 
                    let path = f.path();
                    if path.is_file() {
                        let file_name = f.file_name().to_str().map(|s| s.to_string() );
                        let path_string = path.to_str().map(|s| s.to_string() );

                        if let (Some(l), Some(r)) = (file_name, path_string) {
                            Some((l, r))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None
            }
        }).collect::<Vec<_>>();

        Ok(files)
    }
}

fn on_select_db(s: &mut Cursive, value: &String) {
    let value = value.to_owned();
    let cb_sink = s.cb_sink().clone();
    thread::spawn(move || {
        let db_info = Romst::get_db_info(value);
        let content = match db_info {
            Ok(info) => {
                format!("{}", info)
            }
            Err(e) => {
                format!("Error reading DB details.\n\n{}", e)
            }
        };
        cb_sink.send(Box::new(move |s| {
            s.call_on_name("db_details", |view: &mut TextView| {
                view.set_content(content);
            });
        })).unwrap();
    });
}

fn on_choose_db(s: &mut Cursive, value: &String) {
    let mut browse_db = ListSets::new(value);
    let view = browse_db.load_view();
    match view {
        Ok(v) => {
            s.pop_layer();
            s.add_layer(v);
        }
        Err(e) => {
            let v = browse_db.load_error_dialog(e);
            s.add_layer(v);
        }
    }
}
