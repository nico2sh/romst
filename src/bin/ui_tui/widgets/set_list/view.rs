use romst::data::models::set::GameSet;
use tui::{backend::Backend, layout::{Constraint, Layout, Rect}, style::{Color, Modifier, Style}, text::{Span, Spans, Text}, widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap}};

use crate::ui_tui::widgets::RomstView;

#[derive(Clone)]
pub struct SetEntry {
    pub name: String,
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

pub enum SetDetails {
    None,
    GameSet(GameSet),
    Error(String)
}

pub struct SetListView {
    pub set_list: Vec<SetEntry>,
    pub details: SetDetails,
    pub selected: ListState,
}

impl SetListView {
    pub fn new(set_list: Vec<(String, String)>) -> Self {
        let details = SetDetails::None;
        let selected = ListState::default();

        let set_list = set_list
            .into_iter()
            .map(|item| {
                SetEntry{ name: item.0, desc: item.1 }
            }).collect::<Vec<_>>();
        Self { set_list, details, selected }
    }

    fn get_list_sets_widget<'a>(&self) -> List<'a> {
        let sets = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Sets")
            .border_type(BorderType::Plain);
        let items = self.set_list.iter().map(|entry| ListItem::new(entry.clone())).collect::<Vec<_>>();
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
        let text = if let SetDetails::GameSet(game_set) = &self.details {
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

impl <T: Backend> RomstView<T> for SetListView {
    fn render_in(&mut self, frame: &mut tui::Frame<T>, area: tui::layout::Rect) {
        let chunks = Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .constraints(
                [Constraint::Percentage(20), Constraint::Percentage(80)].as_ref(),
            )
            .split(area);
        
        let list_sets = self.get_list_sets_widget();
        frame.render_stateful_widget(list_sets, chunks[0], &mut self.selected);

        let detail = self.get_db_detail_widget(chunks[1]);
        frame.render_widget(detail, chunks[1]);
    }
}