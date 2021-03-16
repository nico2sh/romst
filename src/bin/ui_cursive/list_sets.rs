use std::{rc::Rc, sync::{Arc, Mutex}, thread};

use cursive::{Cursive, View, align::HAlign, theme::{Effect, Style}, traits::{Boxable, Nameable, Scrollable}, utils::markup::StyledString, views::{Button, Dialog, DummyView, EditView, LinearLayout, Panel, ResizedView, SelectView, TextView}};
use romst::{RomsetMode, Romst, data::{models::{file::DataFile, set::GameSet}, reader::{DataReader, sqlite::DBReader}}};

use anyhow::Result;

use super::utils::{get_style_bad_dump, get_style_no_dump, truncate_text};

pub struct ListSets {
    db_file: String,
    rom_mode: RomsetMode,
    set_list: Vec<(String, String)>
}

impl ListSets {
    pub fn new<S>(db_file: S) -> Self where S: Into<String> {
        Self {
            db_file: db_file.into(),
            rom_mode: RomsetMode::default(),
            set_list: vec![]
        }
    }

    pub fn load_view(&mut self) -> Result<ResizedView<Dialog>> {
        let db_reader = Romst::get_data_reader(&self.db_file)?;

        self.set_list = db_reader.get_game_list(self.rom_mode)?;

        let g_list = self.set_list.iter()
        .map(|item| {
            (format!("{}", truncate_text(&item.0, 20)), item.0.clone())
        }).collect::<Vec<_>>();

        let mut select_game = SelectView::new()
            .h_align(HAlign::Left)
            .autojump();
        select_game.add_all(g_list);

        let db = Arc::new(Mutex::new(db_reader));
        select_game = select_game
        .on_select(move |s, value| {
            on_select_game(s, value.to_owned(), Arc::clone(&db));
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
        let sets = self.set_list.clone();
        let layout = LinearLayout::horizontal()
        .child(Button::new("Filter: [*None*]", move |s| {
            filter_games_dialog(s, sets.clone());
        }).with_name("button_filter"))
        .child(DummyView)
        .child(TextView::new("|"))
        .child(DummyView)
        .child(Button::new(format!("Rom Mode: {}", self.rom_mode), |s| {

        }))
        .child(DummyView)
        .child(TextView::new("|"))
        .child(DummyView)
        .child(Button::new("Scan Directory", |s| {

        }));

        Panel::new(layout).full_width()
    }
}

fn filter_games_dialog<'a>(s: &mut Cursive, set_list: Vec<(String, String)>) {
    let list = Rc::new(set_list.clone());
    let list1 = Rc::clone(&list);
    let filter_dialog = Dialog::new()
    .content(EditView::new()
        .on_submit(move |s, filter| {
            filter_set(s, Rc::clone(&list1), filter);
        })
        .with_name("filter_text")
        .fixed_width(40)
    ).button("Filter", move |s| {
        let content = s.call_on_name("filter_text", |view: &mut EditView| {
            view.get_content()
        });
        if let Some(filter) = content {
            filter_set(s, Rc::clone(&list), &filter);
        };
        s.pop_layer();
    }).button("Close", |s| {
        s.pop_layer();
    });
    s.add_layer(filter_dialog);
}

fn filter_set(s: &mut Cursive, set_list: Rc<Vec<(String, String)>>, filter: &str) {
    let filtered = set_list.iter().filter_map(|set| {
        if set.0.contains(filter) || set.1.contains(filter) {
            Some((format!("{}", truncate_text(&set.0, 20)), set.0.clone()))
        } else {
            None
        }
    }).collect::<Vec<_>>();

    s.call_on_name("selection_list", |view: &mut SelectView<String>| {
        view.clear();
        view.add_all(filtered);
    });

    let filter = if filter.is_empty() { "*None*" } else { filter };
    s.call_on_name("button_filter", |view: &mut Button| {
        view.set_label(format!("Filter: [{}]", filter));
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
