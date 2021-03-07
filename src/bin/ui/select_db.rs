use std::{fs, path::Path};

use cursive::{Cursive, View, align::HAlign, traits::{Boxable, Nameable, Scrollable}, views::{Dialog, DummyView, LinearLayout, Panel, SelectView, TextView}};
use romst::Romst;
use anyhow::Result;

use super::browse_db::BrowseDB;

const BASE_PATH: &str = "db";

pub struct SelectDB {

}

impl SelectDB {
    pub fn new() -> Self {
        Self { }
    }

    pub fn default() -> Self {
        Self::new()
    }

    pub fn load_view(&self) -> Result<impl View> {
        let mut select_db = SelectView::new()
            .h_align(HAlign::Left)
            .autojump()
            .on_select(on_select_db)
            .on_submit(on_choose_db);
        
        let db_list = self.get_db_list()?;

        select_db.add_all(db_list.clone());

        let mut db_details = TextView::new("No DB file found, make sure you have a DB in the `./db` directory next to the Romst executable.")
            .h_align(HAlign::Center);
        if !db_list.is_empty() {
            select_db = select_db.selected(0);

            update_text_view(&mut db_details, &db_list[0].1);
        } else {
            select_db.disable();
        }

        let dialog = Dialog::around(LinearLayout::horizontal()
            .child(select_db.with_name("selection_list").scrollable())
            .child(DummyView)
            .child(Panel::new(db_details.with_name("db_details")).full_width())
            )
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
    s.call_on_name("db_details", |view: &mut TextView| {
        update_text_view(view, value)
    });
}

fn on_choose_db(s: &mut Cursive, value: &String) {
    let browse_db = BrowseDB::new(value);
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

fn update_text_view(view: &mut TextView, value: &String) {
    let db_info = Romst::get_db_info(value);
    match db_info {
        Ok(info) => {
            view.set_content(format!("{}", info))
        }
        Err(e) => {
            view.set_content(format!("Error reading DB details.\n\n{}", e))
        }
    }
}