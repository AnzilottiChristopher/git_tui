use crossterm::event::{self, KeyCode, KeyEventKind};
use octocrab::Octocrab;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::ToSpan,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph},
};
use std::{
    io,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub struct App {
    exit: bool,
    repos: Vec<octocrab::models::Repository>,
    chosen_repo: Option<octocrab::models::Repository>,
    list_state: ListState,
    octocrab: Octocrab,
    selected_readme: Option<String>,
    last_selection_change: Option<Instant>,
    tx: mpsc::Sender<Event>,
    focused_panel: FocusedPanel,
    readme_scroll: u16,
}
pub enum Event {
    Input(crossterm::event::KeyEvent),
    ReadmeFetched(String),
}

#[derive(PartialEq)]
enum FocusedPanel {
    RepoList,
    Description, // This is the ReadMe part not the description
    SingleRepo(SingleRepoPanel),
}

#[derive(PartialEq)]
enum SingleRepoPanel {
    Origin,
    Local,
    Branches,
    TodoList,
}

impl App {
    pub fn new(
        repos: Vec<octocrab::models::Repository>,
        octocrab: Octocrab,
        tx: mpsc::Sender<Event>,
    ) -> Self {
        App {
            exit: false,
            repos,
            chosen_repo: None,
            list_state: ListState::default(),
            octocrab: octocrab,
            selected_readme: None,
            last_selection_change: None,
            tx,
            focused_panel: FocusedPanel::RepoList,
            readme_scroll: 0,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: mpsc::Receiver<Event>,
    ) -> io::Result<()> {
        self.list_state.select_first();
        while !self.exit {
            if let Some(last_change) = self.last_selection_change {
                if last_change.elapsed() >= Duration::from_millis(300) {
                    self.last_selection_change = None;
                    self.on_select();
                }
            }
            match rx.try_recv() {
                Ok(Event::Input(key_event)) => self.check_os(key_event),
                Ok(Event::ReadmeFetched(text)) => self.selected_readme = Some(text),
                Err(_) => {}
            }
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    // This function fires multiple times
    // This checks which os the user is on to determine if a KeyEventKind::Press is needed
    // Linux normally doesn't but some terminals may need it
    pub fn check_os(&mut self, key_event: crossterm::event::KeyEvent) {
        if cfg!(target_os = "windows") {
            if key_event.kind == KeyEventKind::Press {
                match self.focused_panel {
                    FocusedPanel::RepoList => self.handle_key_event_repos(key_event.code),
                    FocusedPanel::Description => self.handle_key_event_description(key_event.code),
                    FocusedPanel::SingleRepo(ref panel) => match panel {
                        //TODO need more specific keys for certain chosen panels
                        _ => self.handle_key_event_single_repo(key_event.code),
                    },
                }
            }
        } else if cfg!(target_os = "linux") {
            match self.focused_panel {
                FocusedPanel::RepoList => self.handle_key_event_repos(key_event.code),
                FocusedPanel::Description => self.handle_key_event_description(key_event.code),
                FocusedPanel::SingleRepo(ref panel) => match panel {
                    //TODO need more specific keys for certain chosen panels
                    _ => self.handle_key_event_single_repo(key_event.code),
                },
            }
        } else {
            //Handle Mac OS here
        }
    }

    fn handle_key_event_single_repo(&mut self, key: crossterm::event::KeyCode) {
        if key == KeyCode::Esc || key == KeyCode::Char('q') {
            self.exit = true;
        }

        match key {
            KeyCode::Char(char) => match char {
                'B' => self.focused_panel = FocusedPanel::RepoList,
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_key_event_description(&mut self, key: crossterm::event::KeyCode) {
        if key == KeyCode::Esc || key == KeyCode::Char('q') {
            self.exit = true;
        }

        match key {
            KeyCode::Char(char) => match char {
                'h' => self.focused_panel = FocusedPanel::RepoList,
                'l' => self.focused_panel = FocusedPanel::Description,
                'j' => self.readme_scroll += 1,
                'k' => self.readme_scroll = self.readme_scroll.saturating_sub(1),
                _ => {}
            },
            KeyCode::Left => self.focused_panel = FocusedPanel::RepoList,
            KeyCode::Right => self.focused_panel = FocusedPanel::Description,
            KeyCode::Down => self.readme_scroll += 1,
            KeyCode::Up => self.readme_scroll = self.readme_scroll.saturating_sub(1),
            _ => {}
        }
    }

    fn handle_key_event_repos(&mut self, key: crossterm::event::KeyCode) {
        if key == KeyCode::Esc || key == KeyCode::Char('q') {
            self.exit = true;
        }

        match key {
            KeyCode::Char(char) => match char {
                'k' => {
                    self.list_state.select_previous();
                    self.last_selection_change = Some(Instant::now());
                    self.selected_readme = None;
                    self.readme_scroll = 0;
                }
                'j' => {
                    self.list_state.select_next();
                    self.last_selection_change = Some(Instant::now());
                    self.selected_readme = None;
                    self.readme_scroll = 0;
                }
                'h' => self.focused_panel = FocusedPanel::RepoList,
                'l' => self.focused_panel = FocusedPanel::Description,
                _ => {}
            },
            KeyCode::Up => {
                self.list_state.select_previous();
                self.last_selection_change = Some(Instant::now());
                self.selected_readme = None;
            }
            KeyCode::Down => {
                self.list_state.select_next();
                self.last_selection_change = Some(Instant::now());
                self.selected_readme = None;
            }
            KeyCode::Enter => {
                self.chosen_repo = self
                    .list_state
                    .selected()
                    .map(|index| self.repos[index].clone());
                self.focused_panel = FocusedPanel::SingleRepo(SingleRepoPanel::Origin)
            }
            KeyCode::Left => self.focused_panel = FocusedPanel::RepoList,
            KeyCode::Right => self.focused_panel = FocusedPanel::Description,
            _ => {}
        }
    }

    pub fn spawn_input_thread(tx: mpsc::Sender<Event>) {
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
    }

    // ALL MY DRAWING/RATATUI Functions
    fn draw(&mut self, frame: &mut Frame) {
        let [mut main_area] = Layout::vertical([Constraint::Percentage(95)]).areas(frame.area());

        if !matches!(self.focused_panel, FocusedPanel::SingleRepo(_)) {
            self.draw_repo_list(frame, main_area);
        } else {
            [main_area] = Layout::vertical([Constraint::Fill(1)]).areas(frame.area());
            self.draw_single_repo(frame, main_area);
        }
    }

    fn draw_single_repo(&mut self, frame: &mut Frame, area: Rect) {
        let name = self
            .chosen_repo
            .as_ref()
            .map(|repo| repo.name.clone())
            .unwrap_or("No Repo Selected".to_string());

        let block = Block::default()
            .title(name.to_span().into_centered_line())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let [file_area, tabs_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(inner_area);

        //Branch Area block
        let mut selection = 0;
    }
    fn draw_origin_files(&mut self, frame: &mut Frame, area: Rect) {}
    fn draw_local_files(&mut self, frame: &mut Frame, area: Rect) {}
    fn render_tab_content(&mut self, frame: &mut Frame, area: Rect, selected_tab: usize) {
        //TODO Fix tabs
        let text: &str = match selected_tab {
            0 => "Tab 1".into(),
            1 => "Tab 2".into(),
            _ => unreachable!(),
        };

        let block = Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::bordered());

        frame.render_widget(block, area);
    }

    fn draw_repo_list(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Github Repositories".to_span().into_centered_line())
            .title_bottom(
                "'←/h-Left' '↓/j-Down' '↑/k-Up' '→/l-Right'"
                    .to_span()
                    .into_centered_line(),
            )
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
            .block(Block::default().borders(Borders::ALL).border_type(
                if self.focused_panel == FocusedPanel::RepoList {
                    BorderType::Double
                } else {
                    BorderType::Rounded
                },
            ))
            .highlight_symbol(">")
            .highlight_style(Style::default().fg(Color::Yellow));

        frame.render_stateful_widget(list, list_area, &mut self.list_state);

        if let Some(index) = self.list_state.selected() {
            let repo = &self.repos[index];

            let repo_name = repo.name.clone();
            let detail_block = Block::default()
                .title(repo_name.to_span().fg(Color::Yellow))
                .padding(Padding::top(1))
                .borders(Borders::ALL)
                .border_type(if self.focused_panel == FocusedPanel::Description {
                    BorderType::Double
                } else {
                    BorderType::Rounded
                });

            let detail_inner = detail_block.inner(detail_area);
            frame.render_widget(detail_block, detail_area);

            let [description_area, readme_area] =
                Layout::vertical([Constraint::Percentage(20), Constraint::Fill(1)])
                    .areas(detail_inner);

            let description = repo
                .description
                .clone()
                .unwrap_or("No Description Available".to_string());

            frame.render_widget(Paragraph::new(description), description_area);

            let readme_text = match &self.selected_readme {
                Some(text) => text.clone(),
                None => "Loading...".to_string(),
            };
            let readme_block = Block::default();

            let readme_inner = readme_block.inner(readme_area);

            frame.render_widget(readme_block, readme_area);
            frame.render_widget(
                Paragraph::new(readme_text).scroll((self.readme_scroll, 0)),
                readme_inner,
            );
        }
    }

    fn on_select(&mut self) {
        if let Some(index) = self.list_state.selected() {
            let repo = &self.repos[index];
            let owner = repo.owner.clone().unwrap().login;
            let repo_name = repo.name.clone();
            let octocrab = self.octocrab.clone();
            let tx = self.tx.clone();

            tokio::spawn(async move {
                if let Ok(content) = octocrab.repos(&owner, &repo_name).get_readme().send().await {
                    let text = content.decoded_content().unwrap_or_default();
                    tx.send(Event::ReadmeFetched(text)).ok();
                }
            });
        }
    }
}
