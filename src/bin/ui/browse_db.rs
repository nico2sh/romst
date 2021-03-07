use std::borrow::Cow;

use cursive::{Cursive, View, align::HAlign, traits::{Boxable, Nameable, Scrollable}, views::{Dialog, DummyView, LinearLayout, Panel, SelectView, TextView}};
use romst::{RomsetMode, Romst, data::models::set::GameSet};

use anyhow::Result;


pub struct BrowseDB {
    db_file: String,
    rom_mode: RomsetMode
}

impl BrowseDB {
    pub fn new<S>(db_file: S) -> Self where S: Into<String> {
        Self { 
            db_file: db_file.into(),
            rom_mode: RomsetMode::default()
        }
    }

    pub fn load_view(&self) -> Result<impl View> {
        let db_file = self.db_file.clone();
        let rom_mode = self.rom_mode.clone();

        let game_list = Romst::get_games(&db_file, rom_mode)?;

        let g_list = game_list.into_iter()
        .map(|item| {
            (format!("{}", truncate_text(&item.0, 14)), item.0)
        }).collect::<Vec<_>>();

        let mut select_game = SelectView::new()
            .h_align(HAlign::Left)
            .autojump()
            .on_select(move |s, value| {
                let result = Romst::get_set_info(&db_file, value, rom_mode);
                update_details_text(s, result);
            });
        select_game.add_all(g_list);

        let mut game_details = TextView::new("")
            .h_align(HAlign::Center);

        // let top_view = 

        let center_view = LinearLayout::horizontal()
            .child(select_game.with_name("selection_list").scrollable())
            .child(DummyView)
            .child(Panel::new(game_details.with_name("game_details")).full_width());

        let dialog = Dialog::around(center_view)
            .title("Select Set")
            .full_screen();

        Ok(dialog)
    }

    pub fn load_error_dialog(&self, e: anyhow::Error) -> impl View {
        Dialog::around(
            TextView::new(format!("Error loading the DB {}\n\n{}", self.db_file, e))
            .h_align(HAlign::Center)
        ).button("Close", |s| { s.pop_layer(); } )
    }
}

fn update_details_text(s: &mut Cursive, result: Result<GameSet>) {
    match result {
        Ok(gs) => {
            s.call_on_name("game_details", |view: &mut TextView| {
                view.set_content(format!("{}", gs));
            });
        }
        Err(e) => {
            s.call_on_name("game_details", |view: &mut TextView| {
                view.set_content(format!("Error\n\n{}", e));
            });
        }
    }
}

fn truncate_text<'a, S>(text: &'a S, len: usize) -> Cow<'a, str> where S: AsRef<str> {
    if text.as_ref().chars().count() <= len {
        return Cow::Borrowed(text.as_ref());
    } else if len == 0 {
        return Cow::Borrowed("");
    }

    let result = text.as_ref()
        .chars()
        .take(len)
        .chain("...".chars())
        .collect();
    Cow::Owned(result)
}