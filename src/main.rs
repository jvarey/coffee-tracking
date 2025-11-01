use std::{io, time::Duration};

use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{palette::tailwind::SLATE, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal,
};
use uuid::Uuid;

const DATE_FMT: &'static str = "%Y/%m/%d %H:%M";
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

fn main() -> io::Result<()> {
    let terminal = ratatui::init();
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
pub struct App {
    state: AppState,
    phase: Phase,
    entries: Vec<Entry>,
    coffees: Vec<Coffee>,
    grinders: Vec<Grinder>,
    exit: bool,
}

#[derive(Debug)]
pub struct AppState {
    entry_list_state: ListState,
    cmd_buffer: String,
    editing_cmd: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(mut self, mut terminal: DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.state.editing_cmd {
            match key_event.code {
                KeyCode::Char(val) => self.state.cmd_buffer.push(val),
                KeyCode::Enter => {
                    self.handle_command(self.state.cmd_buffer.clone());
                    self.state.cmd_buffer.clear();
                    self.state.editing_cmd = false;
                }
                KeyCode::Backspace => {
                    _ = self.state.cmd_buffer.pop();
                    self.state.editing_cmd = !self.state.cmd_buffer.is_empty();
                }
                KeyCode::Esc => {
                    self.state.cmd_buffer.clear();
                    self.state.editing_cmd = false;
                }
                _ => {}
            }
        } else {
            // handle new command input
            if matches!(key_event.code, KeyCode::Char(':')) {
                self.state.cmd_buffer.push(':');
                self.state.editing_cmd = true;
            } else {
                // commands aren't being entered, pass key events on to phase-specific handling
                match self.phase {
                    Phase::ListView => self.handle_key_events_listview(key_event),
                    Phase::EditEntry(_) => self.handle_key_events_editentry(key_event),
                    _ => {}
                }
            }
        }
    }

    fn handle_key_events_editentry(&mut self, key_event: KeyEvent) {
        match key_event.code {
            _ => self.phase = Phase::ListView,
        }
    }

    fn handle_key_events_listview(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('j') => self.state.entry_list_state.select_next(),
            KeyCode::Char('k') => self.state.entry_list_state.select_previous(),
            KeyCode::Char('g') => self.state.entry_list_state.select_first(),
            KeyCode::Enter => {
                if let Some(i) = self.state.entry_list_state.selected() {
                    self.phase = Phase::EditEntry(i);
                }
            }
            _ => {}
        }
    }

    fn handle_command(&mut self, cmd: String) {
        match cmd.as_str() {
            ":q" => self.exit = true,
            _ => {}
        }
    }

    fn render_main(&mut self, area: Rect, buf: &mut Buffer) {
        match self.phase {
            Phase::ListView => self.render_list_view(area, buf),
            Phase::EditEntry(i) => self.render_edit_entry_view(i, area, buf),
            _ => {}
        }
    }

    fn render_edit_entry_view(&mut self, entry_idx: usize, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(self.title())
            .border_set(border::ROUNDED);
        let text: Text = self
            .format_entry_details(&self.entries[entry_idx])
            .into_iter()
            .map(Line::raw)
            .collect();
        Paragraph::new(text).block(block).render(area, buf);
    }

    fn render_list_view(&mut self, area: Rect, buf: &mut Buffer) {
        let entries_text: Vec<String> = self
            .entries
            .iter()
            .map(|e| self.format_entry_item(e))
            .collect();
        let block = Block::bordered()
            .title(self.title())
            .border_set(border::ROUNDED);
        let list = List::new(entries_text)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol("->")
            .block(block);
        StatefulWidget::render(list, area, buf, &mut self.state.entry_list_state);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        match self.phase {
            Phase::ListView => self.render_footer_listview(area, buf),
            _ => {}
        }
    }

    fn render_footer_listview(&self, area: Rect, buf: &mut Buffer) {
        let controls = Line::from(vec![
            " Controls:".into(),
            " Next ".into(),
            "<j>".blue().bold(),
            " | Previous ".into(),
            "<k>".blue().bold(),
            " | Quit ".into(),
            "<q> ".blue().bold(),
        ]);
        let cmd = Line::from(self.state.cmd_buffer.clone());
        Paragraph::new(vec![controls, cmd]).render(area, buf);
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn title(&self) -> String {
        match self.phase {
            Phase::ListView => String::from(" Coffee Tracking - Entries "),
            _ => String::from(" Coffee Tracking "),
        }
    }

    fn format_entry_item(&self, entry: &Entry) -> String {
        let star = if entry.favorite { "*" } else { " " }.bold().blue();
        // let star = if entry.favorite { "★" } else { "☆" }.bold().blue();
        format!(
            " {} {} | {}",
            star,
            entry.dt_taken.format(DATE_FMT),
            &self
                .coffees
                .iter()
                .find(|&c| c.uuid == entry.coffee_id)
                .unwrap()
                .name
        )
    }

    fn format_entry_details(&self, entry: &Entry) -> Vec<String> {
        vec![
            format!("  Date brewed: {}", entry.dt_taken.format(DATE_FMT)),
            format!(
                "  Coffee: {}",
                &self
                    .coffees
                    .iter()
                    .find(|&c| c.uuid == entry.coffee_id)
                    .unwrap()
                    .name
            ),
            format!(
                "  Grinder: {}",
                &self
                    .grinders
                    .iter()
                    .find(|&g| g.uuid == entry.grinder_id)
                    .unwrap()
                    .name
            ),
            format!("  Grind setting: {:.1}", entry.grind_setting),
            format!("  Dose: {:.1} g", entry.dose),
            format!("  Output: {:.1} g ", entry.output),
            format!("  Ratio: {:.1} / 1", entry.output / entry.dose),
            format!("  Duration: {:.1} sec", entry.duration),
        ]
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [main_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(2)]).areas(area);

        self.render_main(main_area, buf);
        self.render_footer(footer_area, buf);
    }
}

#[derive(Debug, Default)]
enum Phase {
    #[default]
    ListView,
    EditEntry(usize),
    EditCoffee,
    EditGrinder,
}

#[derive(Debug, Default)]
struct Entry {
    dt_added: DateTime<Local>,
    dt_taken: DateTime<Local>,
    coffee_id: Uuid,
    grinder_id: Uuid,
    grind_setting: f64,
    duration: f64,
    dose: f64,
    output: f64,
    favorite: bool,
    notes: String,
}

#[derive(Debug, Default)]
struct Coffee {
    name: String,
    uuid: Uuid,
}

impl Coffee {
    fn new(name: String) -> Self {
        Self {
            name,
            uuid: Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Default)]
struct Grinder {
    name: String,
    uuid: Uuid,
}

impl Grinder {
    fn new(name: String) -> Self {
        Self {
            name,
            uuid: Uuid::new_v4(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        let coffees = vec![
            Coffee::new(String::from("B&W FSL28")),
            Coffee::new(String::from("Folgers")),
        ];
        let grinder = Grinder::new(String::from("Niche Zero"));
        let now = Local::now();

        Self {
            state: Default::default(),
            phase: Default::default(),
            entries: vec![
                Entry {
                    dt_taken: now + Duration::from_secs(0),
                    coffee_id: coffees[0].uuid.clone(),
                    grinder_id: grinder.uuid.clone(),
                    dose: 18.0,
                    output: 45.1,
                    duration: 26.0,
                    ..Default::default()
                },
                Entry {
                    dt_taken: now + Duration::from_secs(600),
                    coffee_id: coffees[0].uuid.clone(),
                    grinder_id: grinder.uuid.clone(),
                    dose: 18.0,
                    output: 44.6,
                    duration: 32.1,
                    favorite: true,
                    ..Default::default()
                },
                Entry {
                    dt_taken: now + Duration::from_secs(1580),
                    coffee_id: coffees[1].uuid.clone(),
                    grinder_id: grinder.uuid.clone(),
                    dose: 18.0,
                    output: 43.9,
                    duration: 20.9,
                    ..Default::default()
                },
            ],
            coffees: coffees,
            grinders: vec![grinder],
            exit: Default::default(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            entry_list_state: ListState::default().with_selected(Some(0)),
            cmd_buffer: Default::default(),
            editing_cmd: Default::default(),
        }
    }
}
