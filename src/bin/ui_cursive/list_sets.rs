use std::{rc::Rc, sync::{Arc, Mutex}, thread};

use cursive::{Cursive, View, align::HAlign, theme::{Effect, Style}, traits::{Boxable, Nameable, Scrollable}, utils::markup::StyledString, views::{Button, Dialog, DummyView, EditView, LinearLayout, Panel, ResizedView, SelectView, TextView}};
use romst::{RomsetMode, Romst, data::{models::{file::DataFile, set::GameSet}, reader::{DataReader, sqlite::DBReader}}};

use anyhow::Result;

use super::utils::{get_style_bad_dump, get_style_no_dump, truncate_text};

pub struct ListSets {
    db_reader: Arc<Mutex<DBReader>>,
    rom_mode: RomsetMode,
    filter: Arc<Mutex<String>>,
}

impl ListSets {
    pub fn new<S>(db_file: S) -> Self where S: Into<String> {
        let db_reader = Romst::get_data_reader(db_file.into()).unwrap();
        Self {
            db_reader: Arc::new(Mutex::new(db_reader)),
            rom_mode: RomsetMode::default(),
            filter: Arc::new(Mutex::new(String::new()))
        }
    }

    pub fn load_view(&mut self) -> Result<ResizedView<Dialog>> {
        let g_list = self.db_reader.lock().unwrap().get_game_list(self.rom_mode)?.iter()
        .map(|item| {
            (format!("{}", truncate_text(&item.0, 20)), item.0.clone())
        }).collect::<Vec<_>>();

        let mut select_game = SelectView::new()
            .h_align(HAlign::Left)
            .autojump();
        select_game.add_all(g_list);

        let db = Arc::clone(&self.db_reader);
        let db_select = Arc::clone(&db);
        select_game = select_game
        .on_select(move |s, value| {
            on_select_game(s, value.to_owned(), db_select.clone());
        });

        let mut roms_header = StyledString::styled(" Roms |", Style::none());
        roms_header.append(get_style_no_dump(" No Dump "));
        roms_header.append(StyledString::styled("|", Style::none()));
        roms_header.append(get_style_bad_dump(" Bad Dump"));

        let game_roms = SelectView::<DataFile>::new().h_align(HAlign::Left);
        let game_details = LinearLayout::horizontal()
        .child(TextView::new("").with_name("game_details").full_width())
        .child(LinearLayout::vertical()
        .child(TextView::new(roms_header))
        .child(Panel::new(game_roms.with_name("game_roms").scrollable().fixed_width(40))));

        let top_view = self.get_top_view();

        let center_view = LinearLayout::horizontal()
        .child(select_game.with_name("selection_list").scrollable())
        .child(DummyView)
        .child(Panel::new(game_details.full_width()).full_width());

        let view = LinearLayout::vertical()
        .child(top_view)
        .child(center_view);

        let dialog = Dialog::around(view)
        .title("Select Set")
        .full_screen();

        Ok(dialog)
    }

    pub fn load_error_dialog(&self, e: anyhow::Error) -> impl View {
        Dialog::around(
            TextView::new(format!("Error loading the DB {}\n\n{}", "db_file", e))
            .h_align(HAlign::Center)
        ).button("Close", |s| { s.pop_layer(); } )
    }

    fn get_top_view(&self) -> ResizedView<Panel<LinearLayout>> {
        let db_filter = Arc::clone(&self.db_reader);
        let db_rom_mode = Arc::clone(&self.db_reader);
        let filter = Arc::clone(&self.filter);
        let filter_rom_mode = Arc::clone(&self.filter);
        let rom_mode = self.rom_mode;
        let layout = LinearLayout::horizontal()
        .child(Button::new("Filter: [*None*]", move |s| {
            filter_games_dialog(s, db_filter.clone(), rom_mode, filter.clone());
        }).with_name("button_filter"))
        .child(DummyView)
        .child(TextView::new("|"))
        .child(DummyView)
        .child(Button::new(format!("Rom Mode: {}", self.rom_mode), move |s| {
            rom_mod_dialog(s, db_rom_mode.clone(), rom_mode, filter_rom_mode.clone());
        }))
        .child(DummyView)
        .child(TextView::new("|"))
        .child(DummyView)
        .child(Button::new("Scan Directory", |s| {

        }));

        Panel::new(layout).full_width()
    }
}

fn rom_mod_dialog(s: &mut Cursive, db_reader: Arc<Mutex<DBReader>>, rom_mode: RomsetMode, filter: Arc<Mutex<String>>) {
    
}

fn filter_games_dialog<'a>(s: &mut Cursive, db_reader: Arc<Mutex<DBReader>>, rom_mode: RomsetMode, filter: Arc<Mutex<String>>) {
    let db_reader_button = Arc::clone(&db_reader);
    let filter_button = Arc::clone(&filter);
    let current_filter = {
        filter.lock().unwrap().to_owned()
    };
    let filter_dialog = Dialog::new()
    .content(EditView::new()
        .content(current_filter)
        .on_submit(move |s, filter_to_set| {
            *filter.lock().unwrap() = filter_to_set.to_string();
            filter_set(s, db_reader.clone(), rom_mode, filter_to_set);
            s.pop_layer();
        })
        .with_name("filter_text")
        .fixed_width(40)
    ).button("Filter", move |s| {
        let content = s.call_on_name("filter_text", |view: &mut EditView| {
            view.get_content()
        });
        let filter_to_set = match content {
            Some(filter_to_set) => filter_to_set.to_string(),
            None => String::new()
        };
        *filter_button.lock().unwrap() = filter_to_set.to_string();
        filter_set(s, db_reader_button.clone(), rom_mode, &filter_to_set);
        s.pop_layer();
    }).button("Close", |s| {
        s.pop_layer();
    });
    s.add_layer(filter_dialog);
}

fn filter_set(s: &mut Cursive, db_reader: Arc<Mutex<DBReader>>, rom_mode: RomsetMode, filter: &str) {
    let cb_sink = s.cb_sink().clone();
    let filter = filter.to_string();
    thread::spawn(move || {
        let set_list = db_reader.lock().unwrap().get_game_list(rom_mode).unwrap().iter()
            .map(|item| {
                (format!("{}", truncate_text(&item.0, 20)), item.0.clone())
            }).collect::<Vec<_>>();
        let filtered = set_list.iter().filter_map(|set| {
            if set.0.contains(&filter) || set.1.contains(&filter) {
                Some((format!("{}", truncate_text(&set.0, 20)), set.0.clone()))
            } else {
                None
            }
        }).collect::<Vec<_>>();

        cb_sink.send(Box::new(move |s| {
            s.call_on_name("selection_list", |view: &mut SelectView<String>| {
                view.clear();
                view.add_all(filtered);
            });

            let filter = if filter.is_empty() { "*None*" } else { filter.as_str() };
            s.call_on_name("button_filter", |view: &mut Button| {
                view.set_label(format!("Filter: [{}]", filter));
            });
        })).unwrap();
    });


}

fn on_select_game(s: &mut Cursive, game_name: String, db_reader: Arc<Mutex<DBReader>>) {
    let cb_sink = s.cb_sink().clone();
    thread::spawn(move || {
        let result = db_reader.lock().unwrap().get_set_info(game_name, RomsetMode::NonMerged);
        match result {
            Ok(gs) => {
                cb_sink.send(Box::new(move |s| {
                    s.call_on_name("game_details", |view: &mut TextView| {
                        view.set_content(get_styled_from_game_set(&gs));
                    });
                    s.call_on_name("game_roms", |view: &mut SelectView<DataFile>| {
                        let items = gs.roms.into_iter().map(|rom| {
                            let rom_name = if let Some(status) = &rom.status {
                                match status.as_str() {
                                    "baddump" => {
                                        get_style_bad_dump(&rom.name)
                                    },
                                    "nodump" => {
                                        get_style_no_dump(&rom.name)
                                    },
                                    _ => {
                                        StyledString::styled(format!("{} ({})", &rom.name, status), Style::none())
                                    }
                                }
                            } else {
                                StyledString::styled(&rom.name, Style::none())
                            };
                            (rom_name, rom)
                        }).collect::<Vec<_>>();
                        view.clear();
                        view.add_all(items);
                    });
                })).unwrap();
            }
            Err(e) => {
                cb_sink.send(Box::new(move |s| {
                    s.call_on_name("game_details", |view: &mut TextView| {
                        view.set_content(format!("Error\n\n{}", e));
                    });
                })).unwrap();
            }
        }
    });
}

fn get_styled_from_game_set(game_set: &GameSet) -> StyledString {
    let game = &game_set.game;
    let mut styled = StyledString::styled("Name: ", Effect::Bold);
    styled.append(&game.name);
    if let Some(description) = &game.info_description {
        styled.append(StyledString::styled("\nDescription: ", Effect::Bold));
        styled.append(description);
    }
    if let Some(manufacturer) = &game.info_manufacturer {
        styled.append(StyledString::styled("\nManufacturer: ", Effect::Bold));
        styled.append(manufacturer);
    }
    if let Some(year) = &game.info_year {
        styled.append(StyledString::styled("\nYear: ", Effect::Bold));
        styled.append(year);
    }
    if let Some(clone_of) = &game.clone_of {
        styled.append(StyledString::styled("\nClone of: ", Effect::Bold));
        styled.append(clone_of);
    }
    if let Some(rom_of) = &game.rom_of {
        styled.append(StyledString::styled("\nRom of: ", Effect::Bold));
        styled.append(rom_of);
    }
    if let Some(source_file) = &game.source_file {
        styled.append(StyledString::styled("\nSource File: ", Effect::Bold));
        styled.append(source_file);
    }

    styled
}
