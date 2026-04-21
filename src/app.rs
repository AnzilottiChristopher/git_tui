use crossterm::event::{self, KeyCode, KeyEventKind};
use octocrab::Octocrab;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    text::{Line, ToSpan},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Widget},
};
use std::{char, io, sync::mpsc, thread, time::Duration};

pub struct App {
    exit: bool,
    repos: Vec<octocrab::models::Repository>,
    list_state: ListState,
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
            repos,
            list_state: ListState::default(),
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: mpsc::Receiver<Event>,
    ) -> io::Result<()> {
        while !self.exit {
            match rx.recv().unwrap() {
                Event::Input(key_event) => self.check_os(key_event),
            }
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
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

        match key {
            KeyCode::Char(char) => match char {
                'k' => {
                    self.list_state.select_previous();
                }
                'j' => {
                    self.list_state.select_next();
                }
                _ => {}
            },
            KeyCode::Up => {
                self.list_state.select_previous();
            }
            KeyCode::PageDown => {
                self.list_state.select_next();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn spawn_input_thread() -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(event::Event::Key(key_event)) = event::read() {
                        if tx.send(Event::Input(key_event)).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        rx
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [repo_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());

        self.draw_repo_list(frame, repo_area);
    }

    fn draw_repo_list(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Repos".to_span().into_centered_line())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(inner_area);

        let items: Vec<ListItem> = self
            .repos
            .iter()
            .map(|repo| ListItem::new(repo.name.clone()))
            .collect();

        let list = List::new(items)
            .highlight_symbol(">")
            .highlight_style(Style::default().fg(Color::Yellow));

        frame.render_stateful_widget(list, list_area, &mut self.list_state);

        if let Some(index) = self.list_state.selected() {
            let repo = &self.repos[index];

            let description = repo
                .description
                .clone()
                .unwrap_or("No Description Available".to_string());

            let detail_block = Block::default()
                .title(repo.name.clone())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);

            let detail_inner = detail_block.inner(detail_area);

            frame.render_widget(detail_block, detail_area);
            frame.render_widget(Paragraph::new(description), detail_inner);
        }
    }
}
