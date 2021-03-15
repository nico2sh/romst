mod utils;
mod list_db;
mod list_sets;

use anyhow::Result;
use cursive::{Cursive, align::HAlign, event::Key, views::*};

use self::list_db::SelectDB;

pub fn render() -> Result<()> {
    let mut siv = cursive::default();
    siv.runner().step();

    siv.add_global_callback('q', exit);
    siv.add_global_callback(Key::Esc,exit);

    let select_db = SelectDB::new();

    match select_db.load_view() {
        Ok(view) => {
            siv.add_layer(view);
        }
        Err(e) => {
            siv.add_layer(Dialog::around(
                TextView::new(format!("Error starting the app\n\n{}", e))
                .h_align(HAlign::Center)
            ).button("Close", exit));
        }
    }

    siv.run();

    Ok(())
}

fn exit(s: &mut Cursive) {
    s.quit();
}