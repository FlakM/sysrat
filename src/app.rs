use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::VecDeque,
};

use crate::{
    ProcessExecution,
    event::{AppEvent, Event, EventHandler},
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::{ScrollbarState, TableState},
};

use unicode_width::UnicodeWidthStr;

const ITEM_HEIGHT: usize = 1;

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,

    /// Ring of processes. Bounded to 50
    pub processes: VecDeque<ProcessExecution>,

    /// Event handler.
    pub events: EventHandler,

    /// Table state.
    pub state: RefCell<TableState>,

    /// Scrollbar state.
    pub scroll_state: RefCell<ScrollbarState>,

    pub longest_item_lens: (u16, u16, u16, u16), // order is (timestamp, pid, ppid, command)
}

const MAX_ITEMS_COUNT: usize = 50;

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            longest_item_lens: (10, 4, 4, 20),
            processes: VecDeque::with_capacity(MAX_ITEMS_COUNT),
            events: EventHandler::new(),
            state: RefCell::new(TableState::default().with_selected(0)),
            scroll_state: RefCell::new(ScrollbarState::new(MAX_ITEMS_COUNT * ITEM_HEIGHT)),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub fn handle_events(&mut self) -> color_eyre::Result<()> {
        match self.events.next()? {
            Event::Tick => self.tick(),
            Event::Crossterm(event) => match event {
                crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event)?,
                _ => {}
            },
            Event::App(app_event) => match app_event {
                AppEvent::NewProcess(process) => self.add_process(process),
                AppEvent::Quit => self.quit(),
            },
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Char('j') | KeyCode::Down => self.next_row(),
            KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
            KeyCode::Char('l') | KeyCode::Right => self.next_column(),
            KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn add_process(&mut self, process: ProcessExecution) {
        // guard by max 50 processes
        if self.processes.len() == MAX_ITEMS_COUNT {
            self.processes.pop_front();
        }
        self.processes.push_back(process);
        self.longest_item_lens = constraint_len_calculator(&self.processes);
    }

    pub fn next_row(&mut self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i >= self.processes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.borrow_mut().select(Some(i));
        self.scroll_state = RefCell::new(self.scroll_state.take().position(i * ITEM_HEIGHT));
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    self.processes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.borrow_mut().select(Some(i));
        self.scroll_state = RefCell::new(self.scroll_state.take().position(i * ITEM_HEIGHT));
    }

    pub fn next_column(&mut self) {
        self.state.borrow_mut().select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.borrow_mut().select_previous_column();
    }
}

fn constraint_len_calculator(items: &VecDeque<ProcessExecution>) -> (u16, u16, u16, u16) {
    let timestamp_len = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.timestamp.to_string().as_str()))
        .max()
        .unwrap_or(0);
    let pid_len = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.pid.to_string().as_str()))
        .max()
        .unwrap_or(0);

    let ppid_len = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.ppid.to_string().as_str()))
        .max()
        .unwrap_or(0);

    let command_len = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.command.as_str()))
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (
        timestamp_len as u16,
        pid_len as u16,
        ppid_len as u16,
        command_len as u16,
    )
}
