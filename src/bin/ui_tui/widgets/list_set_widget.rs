use crossterm::event::KeyCode;
use romst::{RomsetMode, Romst, data::{models::set::GameSet, reader::{DataReader, sqlite::DBReader}}};
use tui::{backend::Backend, layout::{Constraint, Layout, Rect}, style::{Color, Modifier, Style}, text::{Span, Spans, Text}, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap, canvas::{self, Canvas}}};

use anyhow::Result;

use super::{RomstWidget, WidgetDispatcher};

#[derive(Clone)]
struct SetEntry {
    name: String,
    desc: String
}

impl <'a> Into<Text<'a>> for SetEntry {
    fn into(self) -> Text<'a> {
        let lines = vec![
            Spans::from(Span::styled(self.name.clone(), Style::default().add_modifier(Modifier::BOLD))),
            Spans::from(Span::styled(self.desc.clone(), Style::default().add_modifier(Modifier::DIM))),
            ];

        lines.into()
    }
}

struct SetList {
    state: ListState,
    entries: Vec<SetEntry>,
    selection: Option<GameSet>,
}

impl SetList {
    fn new(entries: Vec<SetEntry>) -> Self {
        Self {
            state: ListState::default(),
            entries,
            selection: None
        }
    }

}

pub struct SetListWidget {
    reader: DBReader,
    set_list: SetList,
    rom_mode: RomsetMode,
}

impl SetListWidget {
    pub fn new(db_file: String) -> Result<Self> {
        let reader = Romst::get_data_reader(db_file)?;
        let rom_mode = RomsetMode::default();
        let set_list = SetList::new(reader.get_game_list(rom_mode)?
            .into_iter()
            .map(|item| {
                SetEntry{ name: item.0, desc: item.1 }
            }).collect::<Vec<_>>());

        Ok(Self { reader, set_list, rom_mode })
    }

    fn update_selected(&mut self) -> Result<()> {
        if let Some(selected) = self.set_list.state.selected() {
            if let Some(set_entry) = self.set_list.entries.get(selected) {
                let set = self.reader.get_set_info(&set_entry.name, self.rom_mode)?;
                self.set_list.selection = Some(set);
            };
        };
        Ok(())
    }

    fn get_list_sets_widget<'a>(&self) -> List<'a> {
        let sets = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Sets")
            .border_type(BorderType::Plain);
        let items = self.set_list.entries.iter().map(|entry| ListItem::new(entry.clone())).collect::<Vec<_>>();
        List::new(items).block(sets).highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD))
    }

    fn get_db_detail_widget<'a>(&self, area: Rect) -> Paragraph<'a> {
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(70), Constraint::Percentage(30)].as_ref(),
            )
            .split(area);

        let title = "Set Details".to_string();
        let text = if let Some(game_set) = &self.set_list.selection {
            // title = if let Some(desc) = &game_set.game.info_description { desc.to_owned() } else { game_set.game.name.to_owned() };
            let game = &game_set.game;
            let mut text = vec![
                Spans::from(vec![
                    Span::styled(format!("Name: {}", game.name), Style::default().add_modifier(Modifier::BOLD)),
                ])
            ];
            if let Some(desc) = &game.info_description {
                text.push(Spans::from(Span::styled(format!("{}", desc), Style::default().add_modifier(Modifier::ITALIC))));
            }
            if let Some(year) = &game.info_year {
                text.push(Spans::from(Span::styled(format!("Year: {}", year), Style::default())));
            }
            if let Some(clone_of) = &game.clone_of {
                text.push(Spans::from(Span::styled(format!("Clone of: {}", clone_of), Style::default())));
            }
            if let Some(rom_of) = &game.rom_of {
                text.push(Spans::from(Span::styled(format!("Rom of: {}", rom_of), Style::default())));
            }
            if let Some(source_file) = &game.source_file {
                text.push(Spans::from(Span::styled(format!("Source file: {}", source_file), Style::default())));
            }
            text
        } else {
            vec![Spans::from(Span::raw("No Selection"))]
        };

        let paragraph = Paragraph::new(text)
        .block(
            Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title(title)
            .border_type(BorderType::Rounded),
        )
        .wrap(Wrap { trim: true });

        return paragraph;
    }
}

impl <'a, T: Backend> RomstWidget<'a, T> for SetListWidget where T: 'a {
    fn render_in(&mut self, frame: &mut tui::Frame<T>, area: Rect) {
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
            )
            .split(area);
        
        let list_sets = self.get_list_sets_widget();
        frame.render_stateful_widget(list_sets, chunks[0], &mut self.set_list.state);

        let detail = self.get_db_detail_widget(chunks[1]);
        frame.render_widget(detail, chunks[1]);
    }

    fn process_key(&mut self, event: crossterm::event::KeyEvent) {
        match event.code {
            KeyCode::Down => {
                let entries = self.set_list.entries.len();
                if let Some(selected) = self.set_list.state.selected() {
                    if selected >= entries - 1 {
                        self.set_list.state.select(Some(0));
                    } else {
                        self.set_list.state.select(Some(selected + 1));
                    }
                } else {
                    if entries > 0 {
                        self.set_list.state.select(Some(0));
                    }
                };
                self.update_selected();
            },
            KeyCode::Up => {
                let entries = self.set_list.entries.len();
                if let Some(selected) = self.set_list.state.selected() {
                    if selected > 0 {
                        self.set_list.state.select(Some(selected - 1));
                    } else {
                        self.set_list.state.select(Some(entries - 1));
                    }
                } else {
                    if entries > 0 {
                        self.set_list.state.select(Some(0));
                    }
                };
                self.update_selected();
            },
            KeyCode::PageDown => {
                let entries = self.set_list.entries.len();
                if let Some(selected) = self.set_list.state.selected() {
                    let new_value = std::cmp::min(entries - 1, selected + 10);
                    self.set_list.state.select(Some(new_value));
                } else {
                    if entries > 0 {
                        self.set_list.state.select(Some(0));
                    }
                };
                self.update_selected();
            },
            KeyCode::PageUp => {
                let entries = self.set_list.entries.len();
                if let Some(selected) = self.set_list.state.selected() {
                    let new_value = std::cmp::max(0, selected - 10);
                    self.set_list.state.select(Some(new_value));
                } else {
                    if entries > 0 {
                        self.set_list.state.select(Some(0));
                    }
                };
                self.update_selected();
            }
            _ => {}
        }
    }

    fn set_sender(&mut self, sender: WidgetDispatcher<'a, T>) {

    }
}