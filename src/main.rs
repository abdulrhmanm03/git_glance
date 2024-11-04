use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use jwalk::WalkDir;
use ratatui::{
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, List, ListItem, Paragraph},
    Frame,
};
use std::{
    fs::File,
    io::{self, stdout, Write},
    path::{Path, PathBuf},
};

use std::env;

fn main() -> io::Result<()> {
    let home_dir = env::var("HOME").expect("Could not retrieve home directory");
    let base_path = Path::new(&home_dir);

    let repos = find_git_repos(base_path);
    let mut terminal = ratatui::init();
    let mut app = App::new(repos);
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}

#[allow(dead_code)]
#[derive(Clone)]
struct GitRepo {
    name: String,
    path: PathBuf,
}

impl GitRepo {
    fn go_to_dir(&self) -> io::Result<()> {
        let file_path = "dir_path.txt";
        let mut file = File::create(file_path)?;
        writeln!(file, "{}", self.path.display())?;
        Ok(())
    }
}

fn find_git_repos(base_path: &Path) -> Vec<GitRepo> {
    WalkDir::new(base_path)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.join(".git").exists() {
                Some(GitRepo {
                    name: path.file_name()?.to_string_lossy().to_string(),
                    path: path.to_path_buf(),
                })
            } else {
                None
            }
        })
        .collect()
}

enum Modes {
    Normal,
    Edit,
    // Help,
}

struct App {
    search_query: String,
    all_repos: Vec<GitRepo>,
    searched_repos: Vec<GitRepo>,
    mode: Modes,
    selected_repo_index: usize,
    is_running: bool,
}

impl App {
    fn new(repos: Vec<GitRepo>) -> Self {
        App {
            search_query: String::new(),
            all_repos: repos.clone(),
            searched_repos: repos,
            mode: Modes::Normal,
            selected_repo_index: 0,
            is_running: true,
        }
    }

    fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
        loop {
            stdout().flush().unwrap();
            terminal.draw(|frame| self.draw(frame))?;
            let _ = self.events_handler();
            if !self.is_running {
                break;
            }
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.is_running = false;
    }

    fn search(
        &mut self,
        fuzzy_finder: fn(query: &str, items: &[GitRepo], threshold: i64) -> Vec<GitRepo>,
    ) {
        if self.search_query.is_empty() {
            self.searched_repos = self.all_repos.clone();
        } else {
            let threshold = 1;
            self.searched_repos = fuzzy_finder(&self.search_query, &self.all_repos, threshold)
        }
        self.selected_repo_index = 0;
    }

    fn next_repo(&mut self) {
        self.selected_repo_index = (self.selected_repo_index + 1) % self.searched_repos.len();
    }

    fn prev_repo(&mut self) {
        if self.selected_repo_index == 0 {
            self.selected_repo_index = self.searched_repos.len() - 1;
        } else {
            self.selected_repo_index -= 1;
        }
    }

    fn go_to_repo(&self) {
        let _ = self.searched_repos[self.selected_repo_index].go_to_dir();
    }

    fn events_handler(&mut self) -> std::io::Result<bool> {
        if let Event::Key(key) = event::read()? {
            match self.mode {
                Modes::Normal => match key.code {
                    KeyCode::Char('i') => self.mode = Modes::Edit,
                    KeyCode::Char('q') => self.exit(),
                    _ => {}
                },

                Modes::Edit => match key.code {
                    KeyCode::Char(c) => {
                        self.append_char_to_search_query(c);
                        self.search(fuzzy_finder)
                    }
                    KeyCode::Backspace => {
                        self.delete_char_from_search_query();
                        self.search(fuzzy_finder)
                    }
                    KeyCode::Esc => self.mode = Modes::Normal,
                    _ => {}
                },
            }

            match key.code {
                KeyCode::Enter => {
                    self.go_to_repo();
                    self.exit()
                }
                KeyCode::Up => self.prev_repo(),
                KeyCode::Down => self.next_repo(),
                _ => {}
            }
        }
        Ok(false)
    }

    fn append_char_to_search_query(&mut self, ch: char) {
        self.search_query.push(ch);
    }

    fn delete_char_from_search_query(&mut self) {
        self.search_query.pop();
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(3), // the Search box
            Constraint::Min(5),    // the repo list box
            Constraint::Length(1), // the helper keymaps
        ]);
        let [search_area, messages_area, help_area] = vertical.areas(frame.area());

        let search_input = Paragraph::new(self.search_query.clone())
            .style(match self.mode {
                Modes::Normal => Style::default(),
                Modes::Edit => Style::default().fg(Color::Red),
            })
            .block(Block::bordered().title("Search"));
        frame.render_widget(search_input, search_area);

        let repos: Vec<ListItem> = self
            .searched_repos
            .iter()
            .enumerate()
            .map(|(i, repo)| {
                if i == self.selected_repo_index {
                    ListItem::new(repo.name.clone()).red()
                } else {
                    ListItem::new(repo.name.clone())
                }
            })
            .collect();
        let repos = List::new(repos).block(Block::bordered().title("Repos"));
        frame.render_widget(repos, messages_area);

        let normal_mode_msg = vec![
            "Press ".into(),
            "i ".bold(),
            "to inter Edit mode ".into(),
            "q ".bold(),
            "to exit ".into(),
        ];
        let edit_mode_msg = vec![
            "Press ".into(),
            "ESC ".bold(),
            "to exit ".into(),
            "Edit mode".into(),
        ];
        let style = Style::default().add_modifier(Modifier::RAPID_BLINK);
        let text = Text::from(match self.mode {
            Modes::Normal => Line::from(normal_mode_msg),
            Modes::Edit => Line::from(edit_mode_msg),
        })
        .patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);
    }
}

fn fuzzy_finder(query: &str, items: &[GitRepo], threshold: i64) -> Vec<GitRepo> {
    let matcher = SkimMatcherV2::default();
    let mut matched_repos: Vec<(GitRepo, i64)> = items
        .iter()
        .filter_map(|repo| {
            matcher
                .fuzzy_match(&repo.name, query)
                .filter(|&score| score >= threshold)
                .map(|score| (repo.clone(), score))
        })
        .collect();

    matched_repos.sort_by(|(_, score1), (_, score2)| score2.cmp(score1));

    matched_repos.into_iter().map(|(repo, _)| repo).collect()
}
