use std::{io, time::Duration};

use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{palette::tailwind::SLATE, Modifier, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, List, ListState, StatefulWidget, Widget},
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

#[derive(Debug, Default)]
pub struct AppState {
    entry_list_state: ListState,
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
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('j') => self.select_next(),
            KeyCode::Char('k') => self.select_previous(),
            KeyCode::Char('g') => self.select_first(),
            KeyCode::Char('n') => self.select_none(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn format_entry(&self, entry: &Entry) -> String {
        format!(
            " - {} | {}",
            entry.dt_taken.format(DATE_FMT),
            &self
                .coffees
                .iter()
                .find(|&c| c.uuid == entry.coffee_id)
                .unwrap()
                .name
        )
    }

    fn select_next(&mut self) {
        self.state.entry_list_state.select_next();
    }

    fn select_previous(&mut self) {
        self.state.entry_list_state.select_previous();
    }

    fn select_first(&mut self) {
        self.state.entry_list_state.select_first();
    }

    fn select_none(&mut self) {
        self.state.entry_list_state.select(None);
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
                    duration: Duration::from_secs_f64(26.0),
                    ..Default::default()
                },
                Entry {
                    dt_taken: now + Duration::from_secs(600),
                    coffee_id: coffees[0].uuid.clone(),
                    grinder_id: grinder.uuid.clone(),
                    dose: 18.0,
                    output: 44.6,
                    duration: Duration::from_secs_f64(32.1),
                    ..Default::default()
                },
                Entry {
                    dt_taken: now + Duration::from_secs(1580),
                    coffee_id: coffees[1].uuid.clone(),
                    grinder_id: grinder.uuid.clone(),
                    dose: 18.0,
                    output: 43.9,
                    duration: Duration::from_secs_f64(20.9),
                    ..Default::default()
                },
            ],
            coffees: coffees,
            grinders: vec![grinder],
            exit: Default::default(),
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.phase {
            Phase::Menu => {
                let title = " Coffee Tracking ".bold();
                let controls = Line::from(vec![
                    " Controls:".into(),
                    " Next ".into(),
                    "<j>".blue().bold(),
                    " | Previous ".into(),
                    "<k>".blue().bold(),
                    " | Quit ".into(),
                    "<q> ".blue().bold(),
                ]);

                let block = Block::bordered()
                    .title(title)
                    .title_bottom(controls)
                    .border_set(border::THICK);

                let entries_text: Vec<String> =
                    self.entries.iter().map(|e| self.format_entry(e)).collect();

                let list = List::new(entries_text)
                    .block(block)
                    .highlight_style(SELECTED_STYLE)
                    .highlight_symbol(">");
                StatefulWidget::render(list, area, buf, &mut self.state.entry_list_state);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Default)]
enum Phase {
    #[default]
    Menu,
    EditEntry,
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
    duration: Duration,
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
