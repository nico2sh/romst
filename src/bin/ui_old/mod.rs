mod tabs_widget;
mod footer_widget;
mod home_widget;
mod select_db_widget;
mod db_list;

use std::{io, sync::mpsc::{self, Receiver}, thread, time::{Duration, Instant}};

use crossterm::{event::{self, Event as CEvent, KeyCode, KeyEvent}, terminal::{disable_raw_mode, enable_raw_mode}};
use anyhow::Result;
use tui::{Frame, Terminal, backend::{Backend, CrosstermBackend}, layout::{Constraint, Direction, Layout, Rect}, widgets::{ListState}};

use self::{footer_widget::Footer, tabs_widget::{MenuItem, TabsMenu}};

enum Event<I> {
    Input(I),
    Tick,
}

pub trait RomstWidget<T: Backend> {
    fn render_in(&mut self, frame: &mut Frame<T>, area: Rect);
    fn process_key(&mut self, event: KeyEvent);
}

pub fn render() -> Result<()> {
    enable_raw_mode().expect("raw mode");

    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let CEvent::Key(key) = event::read().expect("can read events") {
                    tx.send(Event::Input(key)).expect("can send events");
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Event::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut pet_list_state = ListState::default();
    pet_list_state.select(Some(0));
    let mut tabs_menu = TabsMenu::new();

    loop {
        terminal.draw(|frame| {
            let size = frame.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(2),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            let footer = Footer::new();
            footer.render_in(frame, chunks[2]);

            tabs_menu.render_in(frame, chunks[0]);

            tabs_menu.active_widget.render_in(frame, chunks[1]);

        })?;

        if !event_receiver(&rx, &mut tabs_menu)? {
            disable_raw_mode()?;
            terminal.show_cursor()?;
            break;
        };
    };
    
    Ok(())
}

fn event_receiver<T: Backend>(rx: &Receiver<Event<KeyEvent>>, tabs_menu: &mut TabsMenu<T>) -> Result<bool> {
    match rx.recv()? {
        Event::Input(event) => {
            match event.code {
                KeyCode::Char('q') => {
                    return Ok(false);
                },
                KeyCode::Char('h') => {
                    tabs_menu.select_menu_item(MenuItem::Home);
                },
                KeyCode::Char('d') => {
                    tabs_menu.select_menu_item(MenuItem::Database);
                },
                KeyCode::Char('r') => {
                    tabs_menu.select_menu_item(MenuItem::Roms);
                }
                _ => {
                    tabs_menu.active_widget.process_key(event);
                }
            }
        },
        Event::Tick => {}
    };

    Ok(true)
}
