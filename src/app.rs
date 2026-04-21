use crossterm::event::{self, KeyCode, KeyEventKind};
use octocrab::Octocrab;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    text::Line,
    widgets::Block,
};
use std::{io, sync::mpsc, thread, time::Duration};

pub struct App {
    exit: bool,
    repos: Vec<octocrab::models::Repository>,
}
pub enum Event {
    Input(crossterm::event::KeyEvent),
}

#[derive(PartialEq)]
enum OperatingSystem {
    Windows,
    Linux,
    Mac,
}

impl App {
    pub fn new(repos: Vec<octocrab::models::Repository>) -> Self {
        App {
            exit: false,
            repos: Vec::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal, rx: mpsc::Receiver<Event>) {
        while !self.exit {
            match rx.recv().unwrap() {
                Event::Input(key_event) => self.check_os(key_event),
            }
        }
    }

    pub fn check_os(&mut self, key_event: crossterm::event::KeyEvent) {
        if cfg!(target_os = "windows") {
            if key_event.kind == KeyEventKind::Press {
                self.handle_key_event(key_event.code);
            }
        } else if cfg!(target_os = "linux") {
            self.handle_key_event(key_event.code);
        } else {
            //Handle Mac OS here
        }
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyCode) -> io::Result<()> {
        if key == KeyCode::Esc || key == KeyCode::Char('q') {
            self.exit = true;
        }
        Ok(())
    }
}
