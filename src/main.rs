use std::{io, time::Duration};

use chrono::{DateTime, Local};
// use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{palette::tailwind::SLATE, Color, Modifier, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, List, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal,
};
use tui_input::{backend::crossterm::EventHandler, Input};
use uuid::Uuid;

const DATE_FMT: &'static str = "%Y/%m/%d %H:%M";
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const SELECTED_SYMBOL: &'static str = "->";

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
    command: CommandState,
    edit: EditState,
}

#[derive(Debug, Default)]
struct CommandState {
    buffer: String,
    input_mode: InputMode,
}

#[derive(Debug, Default)]
pub struct EditState {
    list_state: ListState,
    input_mode: InputMode,
    input: Input,
}

#[derive(Debug, Default)]
enum InputMode {
    #[default]
    Normal,
    Editing,
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
        if matches!(self.state.command.input_mode, InputMode::Editing) {
            match key_event.code {
                KeyCode::Char(val) => self.state.command.buffer.push(val),
                KeyCode::Enter => {
                    self.handle_command(self.state.command.buffer.clone());
                    self.state.command.buffer.clear();
                    self.state.command.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    _ = self.state.command.buffer.pop();
                    self.state.command.input_mode = if self.state.command.buffer.is_empty() {
                        InputMode::Normal
                    } else {
                        InputMode::Editing
                    };
                }
                KeyCode::Esc => {
                    self.state.command.buffer.clear();
                    self.state.command.input_mode = InputMode::Normal;
                }
                _ => {}
            }
        } else {
            // handle new command input
            if matches!(key_event.code, KeyCode::Char(':')) {
                self.state.command.buffer.push(':');
                self.state.command.input_mode = InputMode::Editing;
            } else {
                // commands aren't being entered, pass key events on to phase-specific handling
                match self.phase {
                    Phase::ListView => self.handle_key_events_listview(key_event),
                    Phase::EditEntry(idx) => self.handle_key_events_editentry(idx, key_event),
                    _ => {}
                }
            }
        }
    }

    fn handle_key_events_editentry(&mut self, entry_idx: usize, key_event: KeyEvent) {
        match self.state.edit.input_mode {
            InputMode::Normal => match key_event.code {
                KeyCode::Char('q') => self.phase = Phase::ListView,
                KeyCode::Char('j') => self.state.edit.list_state.select_next(),
                KeyCode::Char('k') => self.state.edit.list_state.select_previous(),
                KeyCode::Char('e') => {
                    let field_idx = self.state.edit.list_state.selected().unwrap();
                    match Entry::field_type(field_idx) {
                        FieldType::Date => todo!(),
                        FieldType::CoffeeType => todo!(),
                        FieldType::GrinderType => todo!(),
                        FieldType::ShortString => {
                            self.state.edit.input_mode = InputMode::Editing;
                            self.state.edit.input =
                                Input::new(self.field_val_as_string(entry_idx, field_idx));
                        }
                        FieldType::LongString => todo!(),
                        FieldType::Undefined => {}
                    }
                }
                _ => {}
            },
            InputMode::Editing => {
                if matches!(
                    Entry::field_type(self.state.edit.list_state.selected().unwrap()),
                    FieldType::ShortString
                ) {
                    match key_event.code {
                        KeyCode::Enter => {
                            self.save_input(entry_idx);
                        }
                        _ => {
                            let oldval = self.state.edit.input.value().to_string().clone();
                            _ = self.state.edit.input.handle_event(&Event::Key(key_event));
                            if !valid_float(self.state.edit.input.value())
                                && !self.state.edit.input.value().is_empty()
                            {
                                self.state.edit.input = Input::new(oldval);
                            }
                        }
                    }
                }
            }
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
            Phase::EditCoffee => todo!(),
            Phase::EditGrinder => todo!(),
        }
    }

    fn render_edit_entry_view(&mut self, entry_idx: usize, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title(self.title())
            .border_set(border::ROUNDED);
        let text = self.format_entry_details(&self.entries[entry_idx]);
        match self.state.edit.input_mode {
            InputMode::Normal => {
                let list = List::new(text)
                    .highlight_style(SELECTED_STYLE)
                    .highlight_symbol(SELECTED_SYMBOL)
                    .block(block);
                StatefulWidget::render(list, area, buf, &mut self.state.edit.list_state);
            }
            InputMode::Editing => {
                match Entry::field_type(self.state.edit.list_state.selected().unwrap()) {
                    FieldType::Date => todo!(),
                    FieldType::CoffeeType => todo!(),
                    FieldType::GrinderType => todo!(),
                    FieldType::ShortString => {
                        let inner_area = block.inner(area);
                        block.render(area, buf);
                        for row in 0..9 {
                            let subarea = Rect::new(
                                inner_area.x + (SELECTED_SYMBOL.len() as u16),
                                inner_area.y + (row as u16),
                                inner_area.width,
                                1,
                            );
                            if row == self.state.edit.list_state.selected().unwrap() {
                                // split the string at the :
                                let parts: Vec<&str> = text[row as usize].split(":").collect();
                                let mut label = parts[0].to_string();
                                label.push_str(": ");
                                let rhs = parts[1].to_string();
                                let rhs: Vec<&str> = rhs.trim().split(" ").collect();
                                let units_exist = rhs.len() == 2;
                                // need to split this subarea into three parts: label, input box,
                                // units
                                let line_area = if units_exist {
                                    Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints(vec![
                                            Constraint::Length(label.len() as u16),
                                            Constraint::Length(7),
                                            Constraint::Length(rhs[1].len() as u16),
                                        ])
                                        .flex(Flex::Legacy)
                                        .split(subarea)
                                } else {
                                    Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints(vec![
                                            Constraint::Length(label.len() as u16),
                                            Constraint::Length(7),
                                            Constraint::Length(1),
                                        ])
                                        .flex(Flex::Legacy)
                                        .split(subarea)
                                };
                                Paragraph::new(label).render(line_area[0], buf);
                                Paragraph::new(self.state.edit.input.value())
                                    .style(SELECTED_STYLE)
                                    .render(line_area[1], buf);
                                if units_exist {
                                    let unit_str = format!(" {}", rhs[1]);
                                    Paragraph::new(unit_str).render(line_area[2], buf);
                                }
                            } else {
                                Paragraph::new(text[row as usize].as_str()).render(subarea, buf);
                            }
                        }
                    }
                    FieldType::LongString => todo!(),
                    FieldType::Undefined => {
                        unreachable!("Should never be able to edit an undefined field type")
                    }
                }
            }
        }
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
            .highlight_symbol(SELECTED_SYMBOL)
            .block(block);
        StatefulWidget::render(list, area, buf, &mut self.state.entry_list_state);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        match self.phase {
            Phase::ListView => self.render_footer_listview(area, buf),
            Phase::EditEntry(_) => self.render_footer_editview(area, buf),
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
        let cmd = Line::from(self.state.command.buffer.clone());
        Paragraph::new(vec![controls, cmd]).render(area, buf);
    }

    fn render_footer_editview(&self, area: Rect, buf: &mut Buffer) {
        let controls = Line::from(vec![
            " Controls:".into(),
            " Next ".into(),
            "<j>".blue().bold(),
            " | Previous ".into(),
            "<k>".blue().bold(),
            " | Back ".into(),
            "<q>".blue().bold(),
            " | Edit ".into(),
            "<e> ".blue().bold(),
        ]);
        let cmd = Line::from(self.state.command.buffer.clone());
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
            format!("  Notes: {}", entry.notes),
        ]
    }

    fn field_val_as_string(&self, entry_idx: usize, field_idx: usize) -> String {
        let entry = &self.entries[entry_idx];
        format!(
            "{}",
            match field_idx {
                3 => entry.grind_setting,
                4 => entry.dose,
                5 => entry.output,
                7 => entry.duration,
                _ => 0.0,
            }
        )
    }

    fn save_input(&mut self, entry_idx: usize) {
        match Entry::field_type(self.state.edit.list_state.selected().unwrap()) {
            FieldType::Date => todo!(),
            FieldType::CoffeeType => todo!(),
            FieldType::GrinderType => todo!(),
            FieldType::ShortString => {
                if let Ok(val) = self.state.edit.input.value().parse::<f64>() {
                    match self.state.edit.list_state.selected().unwrap() {
                        3 => self.entries[entry_idx].grind_setting = val,
                        4 => self.entries[entry_idx].dose = val,
                        5 => self.entries[entry_idx].output = val,
                        7 => self.entries[entry_idx].duration = val,
                        _ => {}
                    }
                    self.state.edit.input_mode = InputMode::Normal;
                }
                // let val = self.state.edit.input.value_and_reset();
                // let val: f64 = val.parse().unwrap();
            }
            FieldType::LongString => todo!(),
            FieldType::Undefined => todo!(),
        }
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

enum FieldType {
    Date,
    CoffeeType,
    GrinderType,
    ShortString,
    LongString,
    Undefined,
}

impl Entry {
    fn field_type(i: usize) -> FieldType {
        match i {
            0 => FieldType::Date,
            1 => FieldType::CoffeeType,
            2 => FieldType::GrinderType,
            val if (val > 2 && val != 6 && val != 8) => FieldType::ShortString,
            8 => FieldType::LongString,
            _ => FieldType::Undefined,
        }
    }
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
            command: Default::default(),
            edit: EditState {
                list_state: ListState::default().with_selected(Some(0)),
                ..Default::default()
            },
        }
    }
}

fn valid_float(s: &str) -> bool {
    if let Ok(_) = s.parse::<f64>() {
        true
    } else {
        false
    }
}
