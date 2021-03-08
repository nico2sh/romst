use std::{borrow::Cow, sync::mpsc, thread};

use crossbeam::channel;
use cursive::{Cursive, Vec2, View, align::HAlign, traits::{Boxable, Nameable, Scrollable}, views::{Dialog, DummyView, LinearLayout, Panel, SelectView, TextView}};
use romst::{GameSetsInfo, RomsetMode, Romst, data::models::set::GameSet};

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

        let (tx, rx) = mpsc::channel::<String>();

        let game_list = Romst::get_games(&db_file, rom_mode)?;
        let sets = game_list.len();

        let g_list = game_list.into_iter()
        .map(|item| {
            (format!("{}", truncate_text(&item.0, 14)), item.0)
        }).collect::<Vec<_>>();

        let mut select_game = SelectView::new()
            .h_align(HAlign::Left)
            .autojump();
        select_game.add_all(g_list);
        select_game = select_game
            .on_select(move |s, value| {
                let val = value.clone();
                let db = db_file.clone();
                let txc = tx.clone();
                thread::spawn(move || {
                    let result = Romst::get_set_info(&db, &val, rom_mode);
                    txc.send(format!("{}", result.unwrap()));
                });
                // update_details_text(s, result);
            });

        let game_details = GameDetailsView::new(TextView::new(""), rx);

        let top_view = Panel::new(TextView::new(format!("{} Sets, Rom Mode: {}", sets, rom_mode)).full_width());

        let center_view = LinearLayout::horizontal()
            .child(select_game.with_name("selection_list").scrollable())
            .child(DummyView)
            .child(Panel::new(game_details.with_name("game_details")).full_width());

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
            TextView::new(format!("Error loading the DB {}\n\n{}", self.db_file, e))
            .h_align(HAlign::Center)
        ).button("Close", |s| { s.pop_layer(); } )
    }
}

fn load_game_details() {
    thread::spawn(move || {

    });
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

struct GameDetailsView {
    pub inner: TextView,
    rx: mpsc::Receiver<String>
}

impl GameDetailsView {
    fn new(inner: TextView, rx: mpsc::Receiver<String>) -> Self { Self { inner, rx } }

    fn update(&mut self) {
        while let Ok(content) = self.rx.try_recv() {
            self.inner.set_content(content);
        }
    }
}

impl View for GameDetailsView {
    fn layout(&mut self, _: Vec2) {
        // Before drawing, we'll want to update the buffer
        self.update();
    }

    fn draw(&self, printer: &cursive::Printer) {
        self.inner.draw(printer);
    }

    fn needs_relayout(&self) -> bool {
        self.inner.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        self.inner.required_size(constraint)
    }

    fn on_event(&mut self, e: cursive::event::Event) -> cursive::event::EventResult {
        self.inner.on_event(e)
    }

    fn call_on_any<'a>(&mut self, s: &cursive::view::Selector<'_>, a: cursive::event::AnyCb<'a>) {
        self.inner.call_on_any(s, a)
    }

    fn focus_view(&mut self, s: &cursive::view::Selector<'_>) -> Result<(), cursive::view::ViewNotFound> {
        self.inner.focus_view(s)
    }

    fn take_focus(&mut self, source: cursive::direction::Direction) -> bool {
        self.inner.take_focus(source)
    }

    fn important_area(&self, view_size: Vec2) -> cursive::Rect {
        self.inner.important_area(view_size)
    }

    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }
}