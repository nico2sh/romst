mod utils;
mod list_db;
mod list_sets;

use anyhow::Result;
use cursive::{Cursive, align::HAlign, event::Key, theme::{Color, PaletteColor, Theme}, views::*};

use self::list_db::SelectDB;

pub fn render() -> Result<()> {
    let mut siv = cursive::default();
    let theme = custom_theme_from_cursive(&siv);
    siv.set_theme(theme);

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

fn custom_theme_from_cursive(siv: &Cursive) -> Theme {
    // We'll return the current theme with a small modification.
    let mut theme = siv.current_theme().clone();

    theme.palette[PaletteColor::Background] = Color::TerminalDefault;

    theme
}

fn exit(s: &mut Cursive) {
    s.quit();
}