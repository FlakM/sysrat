use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::VecDeque,
};

use crate::{
    ProcessExecution,
    event::{AppEvent, Event, EventHandler},
    process_service::ProcessService,
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    widgets::{ScrollbarState, TableState},
};

use unicode_width::UnicodeWidthStr;

const ITEM_HEIGHT: usize = 1;

#[derive(Debug)]
pub struct LongestItenLens {
    pub(crate) timestamp: u16,
    pub(crate) username: u16,
    pub(crate) pid: u16,
    pub(crate) ppid: u16,
    pub(crate) comm: u16,
    pub(crate) args: u16,
}

impl Default for LongestItenLens {
    fn default() -> Self {
        Self {
            timestamp: 18,
            username: 10,
            pid: 15,
            ppid: 20,
            comm: 15,
            args: 20,
        }
    }
}

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

    pub longest_item_lens: LongestItenLens,

    pub debug_message: String,

    pub process_service: ProcessService,
}

const MAX_ITEMS_COUNT: usize = 50;

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            longest_item_lens: LongestItenLens::default(),
            processes: VecDeque::with_capacity(MAX_ITEMS_COUNT),
            events: EventHandler::new(),
            state: RefCell::new(TableState::default().with_selected(0)),
            scroll_state: RefCell::new(ScrollbarState::new(MAX_ITEMS_COUNT * ITEM_HEIGHT)),
            debug_message: String::new(),
            process_service: ProcessService::new(),
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
                AppEvent::Print(msg) => self.print_msg(msg),
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

            KeyCode::Enter => {
                let selected = self.state.borrow().selected();
                if let Some(i) = selected {
                    let process = self.processes.get(i).unwrap();
                    let pid = process.pid;
                    let ppid = process.ppid;

                    let process = self.process_service.get_process(pid as usize);
                    let parent_process = self.process_service.get_process(ppid as usize);

                    let msg = format!("pid: {:?}\n ppid: {:?}", process, parent_process);

                    self.print_msg(msg);
                }
            }
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

    pub fn print_msg(&mut self, msg: String) {
        self.debug_message = msg;
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

fn constraint_len_calculator(items: &VecDeque<ProcessExecution>) -> LongestItenLens {
    let timestamp_len = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.timestamp.to_string().as_str()))
        .max()
        .unwrap_or(0);
    let pid_len = 6;

    let ppid_len = 15;

    let username_len = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.username.as_deref().unwrap_or_default()))
        .max()
        .unwrap_or(1);

    let comm = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.comm.as_str()))
        .max()
        .unwrap_or(0);

    let args = items
        .iter()
        .map(|d| UnicodeWidthStr::width(d.args.as_str()))
        .max()
        .unwrap_or(0);

    LongestItenLens {
        timestamp: timestamp_len as u16,
        username: username_len as u16,
        pid: pid_len as u16,
        ppid: ppid_len as u16,
        comm: comm as u16,
        args: args as u16,
    }

}
