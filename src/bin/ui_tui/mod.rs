mod widgets;
mod footer_widget;

use std::{io, sync::mpsc::{self, Receiver}, thread, time::{Duration, Instant}};

use crossterm::{event::{self, Event as CEvent, KeyCode, KeyEvent}, terminal::{disable_raw_mode, enable_raw_mode}};
use anyhow::Result;
use tui::{Terminal, backend::{Backend, CrosstermBackend}, layout::{Constraint, Direction, Layout}, widgets::{ListState}};

use self::{footer_widget::Footer, widgets::{ControllerManager, MenuItem, main_widget::{TabsMenu}}};

enum Event<I> {
    Input(I),
    Tick,
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

    let mut controller = ControllerManager::new();
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

            controller.update(frame, chunks[0], chunks[1]);
        })?;

        if !event_receiver(&rx, &mut controller)? {
            disable_raw_mode()?;
            terminal.show_cursor()?;
            break;
        };
    };
    
    Ok(())
}

fn event_receiver<T: Backend>(rx: &Receiver<Event<KeyEvent>>, controller: &mut ControllerManager<T>) -> Result<bool> {
    match rx.recv()? {
        Event::Input(event) => {
            match event.code {
                KeyCode::Char('q') => {
                    return Ok(false);
                },
                KeyCode::Char('h') => {
                    controller.select_menu_item(MenuItem::Home);
                },
                KeyCode::Char('d') => {
                    controller.select_menu_item(MenuItem::Database);
                },
                KeyCode::Char('g') => {
                    controller.select_menu_item(MenuItem::Sets);
                }
                _ => {
                    controller.process_keys_active_controller(event);
                }
            }
        },
        Event::Tick => {}
    };

    Ok(true)
}
