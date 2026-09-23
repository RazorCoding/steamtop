mod games;
mod gpu;
mod metrics;
mod ui;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use games::{collect_games, Game};
use gpu::GpuInfo;
use metrics::Metrics;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Games,
    AllProcs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortCol {
    Cpu,
    Mem,
    Gpu,
    Name,
}

impl SortCol {
    pub fn label(self) -> &'static str {
        match self {
            SortCol::Cpu => "cpu",
            SortCol::Mem => "mem",
            SortCol::Gpu => "gpu",
            SortCol::Name => "name",
        }
    }
}

#[derive(Clone, Debug)]
pub struct DisplayRow {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem_bytes: u64,
    pub pinned: bool,
    pub steam: bool,
}

impl From<&Game> for DisplayRow {
    fn from(g: &Game) -> Self {
        DisplayRow {
            pid: g.pid,
            name: g.name.clone(),
            cpu: g.cpu,
            mem_bytes: g.mem_bytes,
            pinned: g.pinned,
            steam: g.steam,
        }
    }
}

pub struct App {
    pub interval: Duration,
    pub view: View,
    pub sort: SortCol,
    pub selected: usize,
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub gpu: GpuInfo,
    pub pinned: HashSet<u32>,
    pub watch_names: HashMap<u32, String>,
    games_rows: Vec<DisplayRow>,
    procs: Vec<DisplayRow>,
}

impl App {
    fn new() -> Self {
        Self {
            interval: Duration::from_millis(1000),
            view: View::Games,
            sort: SortCol::Cpu,
            selected: 0,
            cpu: 0.0,
            mem_used: 0,
            mem_total: 0,
            gpu: GpuInfo::probe(),
            pinned: HashSet::new(),
            watch_names: HashMap::new(),
            games_rows: Vec::new(),
            procs: Vec::new(),
        }
    }

    pub fn display_rows(&self) -> &[DisplayRow] {
        match self.view {
            View::Games => &self.games_rows,
            View::AllProcs => &self.procs,
        }
    }

    fn refresh(&mut self, metrics: &mut Metrics) {
        metrics.refresh();
        self.cpu = metrics.global_cpu();
        let (used, total) = metrics.memory();
        self.mem_used = used;
        self.mem_total = total;
        self.gpu = GpuInfo::probe();

        let games: Vec<Game> = collect_games(metrics, &self.pinned, &self.watch_names);
        self.games_rows = games.iter().map(DisplayRow::from).collect();

        self.procs = metrics
            .processes()
            .iter()
            .map(|(pid, p)| DisplayRow {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                cpu: p.cpu_usage(),
                mem_bytes: p.memory(),
                pinned: self.pinned.contains(&pid.as_u32()),
                steam: false,
            })
            .collect();

        self.sort_rows();

        let len = self.display_rows().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    fn sort_rows(&mut self) {
        let desc = |a: &f32, b: &f32| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal);
        self.games_rows.sort_by(|a, b| match self.sort {
            SortCol::Cpu => desc(&a.cpu, &b.cpu),
            SortCol::Mem => b.mem_bytes.cmp(&a.mem_bytes),
            SortCol::Gpu => desc(&a.cpu, &b.cpu),
            SortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        self.procs.sort_by(|a, b| match self.sort {
            SortCol::Cpu => desc(&a.cpu, &b.cpu),
            SortCol::Mem => b.mem_bytes.cmp(&a.mem_bytes),
            SortCol::Gpu => desc(&a.cpu, &b.cpu),
            SortCol::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
    }

    fn row_len(&self) -> usize {
        self.display_rows().len()
    }

    fn selected_pid(&self) -> Option<u32> {
        self.display_rows().get(self.selected).map(|r| r.pid)
    }

    fn selected_name(&self) -> Option<String> {
        self.display_rows().get(self.selected).map(|r| r.name.clone())
    }

    fn toggle_pin(&mut self) {
        let (Some(pid), Some(name)) = (self.selected_pid(), self.selected_name()) else {
            return;
        };
        if self.pinned.contains(&pid) {
            self.pinned.remove(&pid);
            self.watch_names.remove(&pid);
        } else {
            self.pinned.insert(pid);
            self.watch_names.insert(pid, name);
        }
    }

    fn adjust_rate(&mut self, up: bool) {
        let secs = self.interval.as_secs_f32();
        let next = if up { secs + 0.25 } else { secs - 0.25 };
        self.interval = Duration::from_secs_f32(next.clamp(0.25, 5.0));
    }
}

fn handle_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('q') => return false,
        KeyCode::Tab => {
            app.view = match app.view {
                View::Games => View::AllProcs,
                View::AllProcs => View::Games,
            };
            app.selected = 0;
        }
        KeyCode::Down => {
            if app.selected + 1 < app.row_len() {
                app.selected += 1;
            }
        }
        KeyCode::Up => {
            app.selected = app.selected.saturating_sub(1);
        }
        KeyCode::Char(' ') => app.toggle_pin(),
        KeyCode::Char('1') => app.sort = SortCol::Cpu,
        KeyCode::Char('2') => app.sort = SortCol::Mem,
        KeyCode::Char('3') => app.sort = SortCol::Gpu,
        KeyCode::Char('4') => app.sort = SortCol::Name,
        KeyCode::Char('+') | KeyCode::Char('=') => app.adjust_rate(true),
        KeyCode::Char('-') | KeyCode::Char('_') => app.adjust_rate(false),
        _ => {}
    }
    true
}

fn main() -> Result<()> {
    let mut metrics = Metrics::init()?;
    let mut app = App::new();
    app.refresh(&mut metrics);

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut last_tick = Instant::now();
    let result = (|| -> Result<()> {
        loop {
            let wait = app
                .interval
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::ZERO);

            if event::poll(wait)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && !handle_key(&mut app, key.code) {
                        break;
                    }
                }
            }

            if last_tick.elapsed() >= app.interval {
                app.refresh(&mut metrics);
                last_tick = Instant::now();
            }

            terminal.draw(|frame| ui::draw(&app, frame))?;
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}
