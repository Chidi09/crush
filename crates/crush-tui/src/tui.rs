use std::collections::{HashMap, VecDeque};
use std::io;
use std::time::{Duration, SystemTime};
use std::sync::OnceLock;
use std::sync::Mutex;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Sparkline, Table, TableState, Tabs},
    Frame, Terminal,
};
use crush_types::{Container, ContainerStatus};

// ─── Brand palette ────────────────────────────────────────────────────────────
const ORANGE:       Color = Color::Rgb(255, 140,   0);
const DIM_ORANGE:   Color = Color::Rgb(180,  95,   0);
const BORDER:       Color = Color::Rgb( 55,  55,  65);
const BORDER_HOT:   Color = Color::Rgb(255, 140,   0);
const DIM:          Color = Color::Rgb( 95,  95, 105);
const SEL_BG:       Color = Color::Rgb( 38,  32,  50);
const RUN_GREEN:    Color = Color::Rgb( 80, 210, 100);
const STOP_RED:     Color = Color::Rgb(220,  65,  65);
const PAUSE_YELLOW: Color = Color::Rgb(220, 185,  60);
const CYAN:         Color = Color::Rgb( 60, 185, 225);

// ─── Procfs Stats Helper on Linux ─────────────────────────────────────────────

struct PidTick {
    ticks: u64,
    time: std::time::Instant,
}

fn get_pid_cache() -> &'static Mutex<HashMap<u32, PidTick>> {
    static CACHE: OnceLock<Mutex<HashMap<u32, PidTick>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "linux")]
fn get_linux_proc_metrics(pid: u32) -> (f32, f32) {
    let statm_path = format!("/proc/{}/statm", pid);
    let mut mem_mb = 0.0f32;
    if let Ok(content) = std::fs::read_to_string(&statm_path) {
        let fields: Vec<&str> = content.split_whitespace().collect();
        if fields.len() > 1 {
            if let Ok(pages) = fields[1].parse::<u64>() {
                mem_mb = (pages * 4096) as f32 / (1024.0 * 1024.0);
            }
        }
    }

    let stat_path = format!("/proc/{}/stat", pid);
    let mut cpu_pct = 0.0f32;
    if let Ok(content) = std::fs::read_to_string(&stat_path) {
        let fields: Vec<&str> = content.split_whitespace().collect();
        if fields.len() > 14 {
            let utime = fields[13].parse::<u64>().unwrap_or(0);
            let stime = fields[14].parse::<u64>().unwrap_or(0);
            let total_process_ticks = utime + stime;

            if let Ok(mut cache) = get_pid_cache().lock() {
                let now = std::time::Instant::now();
                if let Some(prev) = cache.get(&pid) {
                    let ticks_diff = total_process_ticks.saturating_sub(prev.ticks);
                    let time_diff = now.duration_since(prev.time).as_secs_f32().max(0.01);
                    cpu_pct = ((ticks_diff as f32 / 100.0) / time_diff) * 100.0;
                    cpu_pct = cpu_pct.min(100.0).max(0.0);
                }
                cache.insert(pid, PidTick { ticks: total_process_ticks, time: now });
            }
        }
    }

    (cpu_pct, mem_mb)
}

struct IoPidTick {
    rx: u64,
    tx: u64,
    dr: u64,
    dw: u64,
    time: std::time::Instant,
}

fn get_io_cache() -> &'static Mutex<HashMap<u32, IoPidTick>> {
    static CACHE: OnceLock<Mutex<HashMap<u32, IoPidTick>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_io_rates(_pid: u32) -> (f32, f32, f32, f32) {
    #[cfg(target_os = "linux")]
    {
        let mut rx_total: u64 = 0;
        let mut tx_total: u64 = 0;
        let mut dr_total: u64 = 0;
        let mut dw_total: u64 = 0;

        // /proc/<pid>/net/dev: lines like "  eth0: rx_bytes ... tx_bytes ..."
        if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/net/dev", _pid)) {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    rx_total += parts[1].parse::<u64>().unwrap_or(0);
                    tx_total += parts[9].parse::<u64>().unwrap_or(0);
                }
            }
        }

        // /proc/<pid>/io: read_bytes / write_bytes
        if let Ok(content) = std::fs::read_to_string(format!("/proc/{}/io", _pid)) {
            for line in content.lines() {
                if line.starts_with("read_bytes:") {
                    dr_total = line.split(':').nth(1).unwrap_or("0").trim().parse().unwrap_or(0);
                } else if line.starts_with("write_bytes:") {
                    dw_total = line.split(':').nth(1).unwrap_or("0").trim().parse().unwrap_or(0);
                }
            }
        }

        let now = std::time::Instant::now();
        if let Ok(mut cache) = get_io_cache().lock() {
            let rates = if let Some(prev) = cache.get(&_pid) {
                let dt = now.duration_since(prev.time).as_secs_f32().max(0.01);
                (
                    (rx_total.saturating_sub(prev.rx) as f32 / dt),
                    (tx_total.saturating_sub(prev.tx) as f32 / dt),
                    (dr_total.saturating_sub(prev.dr) as f32 / dt),
                    (dw_total.saturating_sub(prev.dw) as f32 / dt),
                )
            } else { (0.0, 0.0, 0.0, 0.0) };
            cache.insert(_pid, IoPidTick { rx: rx_total, tx: tx_total, dr: dr_total, dw: dw_total, time: now });
            return rates;
        }
        (0.0, 0.0, 0.0, 0.0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        (0.0, 0.0, 0.0, 0.0)
    }
}

fn format_rate(bytes_per_sec: f32) -> String {
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.1}GB/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1_048_576.0 {
        format!("{:.1}MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.0}KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0}B/s", bytes_per_sec)
    }
}

// ─── Public types (existing API preserved) ────────────────────────────────────

pub struct AppState {
    pub containers: Vec<Container>,
    pub cpu_history: HashMap<String, VecDeque<f32>>,
    pub mem_history: HashMap<String, VecDeque<f32>>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub running: bool,
    pub mode: AppMode,
}

pub enum AppMode {
    Ps,
    Stats,
    Compose,
    Logs { container_id: String, follow: bool, search: Option<String> },
    Debug,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
            cpu_history: HashMap::new(),
            mem_history: HashMap::new(),
            selected: 0,
            scroll_offset: 0,
            running: true,
            mode: AppMode::Ps,
        }
    }

    pub fn update_containers(&mut self, containers: Vec<Container>) {
        self.containers = containers;
    }

    pub fn record_metrics(&mut self, container_id: &str, cpu_pct: f32, mem_mb: f32) {
        let cpu = self.cpu_history.entry(container_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(60));
        cpu.push_back(cpu_pct);
        if cpu.len() > 60 { cpu.pop_front(); }

        let mem = self.mem_history.entry(container_id.to_string())
            .or_insert_with(|| VecDeque::with_capacity(60));
        mem.push_back(mem_mb);
        if mem.len() > 60 { mem.pop_front(); }
    }

    pub fn container_count(&self) -> usize { self.containers.len() }
    pub fn running_count(&self) -> usize {
        self.containers.iter()
            .filter(|c| matches!(c.status, ContainerStatus::Running))
            .count()
    }
}

pub struct SparklineRenderer;

impl SparklineRenderer {
    pub fn render(data: &VecDeque<f32>, width: usize) -> Vec<&'static str> {
        let chars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        if data.is_empty() { return vec!["▁"; width]; }
        let max = data.iter().cloned().fold(0.0f32, f32::max).max(0.01);
        data.iter().take(width).map(|&v| {
            let idx = ((v / max) * 7.0) as usize;
            chars[idx.min(7)]
        }).collect()
    }

    pub fn render_dots(data: &VecDeque<f32>, width: usize) -> String {
        Self::render(data, width).join("")
    }
}

pub fn status_icon(status: &ContainerStatus) -> &'static str {
    match status {
        ContainerStatus::Running               => "●",
        ContainerStatus::Paused                => "⏸",
        ContainerStatus::Stopped               => "○",
        ContainerStatus::Creating |
        ContainerStatus::Created               => "◌",
    }
}

pub fn format_duration(created: SystemTime) -> String {
    match SystemTime::now().duration_since(created) {
        Ok(d) if d.as_secs() < 60    => format!("{}s",  d.as_secs()),
        Ok(d) if d.as_secs() < 3600  => format!("{}m",  d.as_secs() / 60),
        Ok(d) if d.as_secs() < 86400 => format!("{}h",  d.as_secs() / 3600),
        Ok(d)                         => format!("{}d",  d.as_secs() / 86400),
        Err(_)                        => "--".to_string(),
    }
}

// ─── Interactive app state ─────────────────────────────────────────────────────

#[derive(Clone)]
struct ComposeService {
    name: String,
    status: String,
    image: String,
}

struct App {
    containers: Vec<Container>,
    cpu_history: HashMap<String, VecDeque<f32>>,
    mem_history: HashMap<String, VecDeque<f32>>,
    net_rx_history: HashMap<String, VecDeque<f32>>,
    net_tx_history: HashMap<String, VecDeque<f32>>,
    disk_r_history: HashMap<String, VecDeque<f32>>,
    disk_w_history: HashMap<String, VecDeque<f32>>,
    table_state: TableState,
    active_tab: usize,
    tick: u64,
    data_dir: std::path::PathBuf,

    // Logs view state
    log_scroll: usize,
    follow: bool,
    search_query: String,
    searching: bool,
    search_matches: Vec<usize>,
    active_search_match: usize,

    // Compose view state
    compose_services: Vec<ComposeService>,
    compose_logs: Vec<String>,

    // Debug view state: (source snippet, AI diagnosis text).
    // None = no container selected or diagnosis not yet run.
    debug_info: Option<(String, String)>,

    // When opened via `crush logs <id>`, this pins the initial selection.
    pinned_container_id: Option<String>,
}

impl App {
    fn new(
        containers: Vec<Container>,
        initial_tab: usize,
        data_dir: std::path::PathBuf,
        debug_info: Option<(String, String)>,
        pinned_container_id: Option<String>,
    ) -> Self {
        let mut table_state = TableState::default();
        // If a specific container is pinned (e.g. `crush logs <id>`), select it;
        // otherwise default to the first row.
        let initial_selection = pinned_container_id.as_deref()
            .and_then(|id| containers.iter().position(|c| c.id == id || c.name == id))
            .or_else(|| if !containers.is_empty() { Some(0) } else { None });
        table_state.select(initial_selection);

        let mut cpu_history: HashMap<String, VecDeque<f32>> = HashMap::new();
        let mut mem_history: HashMap<String, VecDeque<f32>> = HashMap::new();
        for (i, c) in containers.iter().enumerate() {
            let cpu: VecDeque<f32> = (0..30u32).map(|j| {
                let base = (i as f32 + 1.0) * 9.0;
                (base + (j as f32 * 0.6 + i as f32 * 1.4).sin().abs() * base * 0.45).min(99.0)
            }).collect();
            let mem: VecDeque<f32> = (0..30u32).map(|j| {
                let base = (i as f32 + 1.0) * 70.0;
                (base + (j as f32 * 0.3 + i as f32 * 2.1).cos().abs() * 25.0).min(512.0)
            }).collect();
            cpu_history.insert(c.id.clone(), cpu);
            mem_history.insert(c.id.clone(), mem);
        }

        let compose_services = Self::load_native_services(&data_dir);
        let compose_logs: Vec<String> = if compose_services.is_empty() {
            vec!["  (no services running — start something with `crush` in a project directory)".to_string()]
        } else {
            Vec::new()
        };

        Self {
            containers,
            cpu_history,
            mem_history,
            net_rx_history: HashMap::new(),
            net_tx_history: HashMap::new(),
            disk_r_history: HashMap::new(),
            disk_w_history: HashMap::new(),
            table_state,
            active_tab: initial_tab,
            tick: 0,
            data_dir,
            log_scroll: 0,
            follow: true,
            search_query: String::new(),
            searching: false,
            search_matches: Vec::new(),
            active_search_match: 0,
            compose_services,
            compose_logs,
            debug_info,
            pinned_container_id,
        }
    }

    /// Load running native dep services from `<data_dir>/native/*.json`.
    /// Each file is a `NativeServiceState` JSON with a `services` array of
    /// `{ name, pid, port, kind }`. We probe each pid to set Running/Stopped.
    fn load_native_services(data_dir: &std::path::Path) -> Vec<ComposeService> {
        let native_dir = data_dir.join("native");
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(&native_dir) {
            Ok(e) => e,
            Err(_) => return out,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
            let content = match std::fs::read_to_string(&p) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let v: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(svcs) = v.get("services").and_then(|s| s.as_array()) {
                for s in svcs {
                    let name = s.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string();
                    let port = s.get("port").and_then(|n| n.as_u64()).unwrap_or(0);
                    let kind = s.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
                    let pid = s.get("pid").and_then(|n| n.as_u64()).unwrap_or(0) as u32;
                    let alive = Self::pid_alive(pid);
                    out.push(ComposeService {
                        name,
                        status: if alive { "Running".to_string() } else { "Stopped".to_string() },
                        image: format!("{} :{}", kind.to_lowercase(), port),
                    });
                }
            }
        }
        out
    }

    #[cfg(windows)]
    fn pid_alive(pid: u32) -> bool {
        if pid == 0 { return false; }
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, GetExitCodeProcess};
        use windows_sys::Win32::Foundation::CloseHandle;
        const STILL_ACTIVE: u32 = 259;
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h == 0 { return false; }
            let mut code: u32 = 0;
            GetExitCodeProcess(h, &mut code);
            CloseHandle(h);
            code == STILL_ACTIVE
        }
    }

    #[cfg(unix)]
    fn pid_alive(pid: u32) -> bool {
        if pid == 0 { return false; }
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    #[cfg(not(any(windows, unix)))]
    fn pid_alive(_pid: u32) -> bool { false }

    fn next_row(&mut self) {
        if self.containers.is_empty() { return; }
        let n = self.containers.len();
        let i = self.table_state.selected().map_or(0, |i| (i + 1) % n);
        self.table_state.select(Some(i));
    }

    fn prev_row(&mut self) {
        if self.containers.is_empty() { return; }
        let n = self.containers.len();
        let i = self.table_state.selected().map_or(0, |i| if i == 0 { n - 1 } else { i - 1 });
        self.table_state.select(Some(i));
    }

    fn selected(&self) -> Option<&Container> {
        self.table_state.selected().and_then(|i| self.containers.get(i))
    }

    fn reload_containers(&mut self) {
        let containers_dir = self.data_dir.join("containers");
        let mut new_containers = Vec::new();
        if containers_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&containers_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let json_path = entry.path().join("container.json");
                        if json_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_path) {
                                if let Ok(c) = serde_json::from_str::<Container>(&content) {
                                    new_containers.push(c);
                                }
                            }
                        }
                    }
                }
            }
        }
        if !new_containers.is_empty() {
            self.containers = new_containers;
        }
    }

    fn on_tick(&mut self) {
        self.tick += 1;
        self.reload_containers();

        for c in &self.containers {
            let (cpu_now, mem_now) = if let Some(pid) = c.pid {
                #[cfg(target_os = "linux")]
                {
                    get_linux_proc_metrics(pid)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let t = self.tick as f32 * 0.14;
                    let cpu = ((t + (pid as f32 % 5.0) * 1.3).sin().abs() * 28.0 + 6.0).min(99.0);
                    let mem = ((t * 0.4 + (pid as f32 % 5.0) * 0.8).cos().abs() * 55.0 + 55.0).min(512.0);
                    (cpu, mem)
                }
            } else {
                (0.0, 0.0)
            };

            let h = self.cpu_history.entry(c.id.clone()).or_default();
            h.push_back(cpu_now);
            if h.len() > 60 { h.pop_front(); }

            let m = self.mem_history.entry(c.id.clone()).or_default();
            m.push_back(mem_now);
            if m.len() > 60 { m.pop_front(); }

            // Net / disk metrics from /proc/<pid>/net/dev and /proc/<pid>/io
            let (rx_rate, tx_rate, dr_rate, dw_rate) = if let Some(pid) = c.pid {
                get_io_rates(pid)
            } else { (0.0, 0.0, 0.0, 0.0) };

            let push_metric = |map: &mut HashMap<String, VecDeque<f32>>, key: &str, val: f32| {
                let h = map.entry(key.to_string()).or_default();
                h.push_back(val);
                if h.len() > 60 { h.pop_front(); }
            };
            push_metric(&mut self.net_rx_history, &c.id, rx_rate);
            push_metric(&mut self.net_tx_history, &c.id, tx_rate);
            push_metric(&mut self.disk_r_history, &c.id, dr_rate);
            push_metric(&mut self.disk_w_history, &c.id, dw_rate);
        }
    }

    fn get_streaming_logs(&self, container_id: &str) -> Vec<String> {
        let dir = self.data_dir.join("containers").join(container_id);
        let mut lines = Vec::new();
        // Try both naming conventions: crush-run.* (stateless engine) and
        // stdout/stderr.log (older path). Non-empty file wins.
        for (stdout_name, stderr_name) in &[
            ("crush-run.log", "crush-run.err"),
            ("stdout.log",    "stderr.log"),
        ] {
            let out = dir.join(stdout_name);
            let err = dir.join(stderr_name);
            let mut batch = Vec::new();
            if let Ok(c) = std::fs::read_to_string(&out) {
                for l in c.lines() { batch.push(format!("[out] {}", l)); }
            }
            if let Ok(c) = std::fs::read_to_string(&err) {
                for l in c.lines() { batch.push(format!("[err] {}", l)); }
            }
            if !batch.is_empty() {
                lines = batch;
                break;
            }
        }
        if lines.is_empty() {
            vec!["  (no logs yet — container may still be starting)".to_string()]
        } else {
            lines
        }
    }
}

// ─── Rendering ────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, area: Rect, app: &mut App) {
    let running = app.containers.iter()
        .filter(|c| matches!(c.status, ContainerStatus::Running))
        .count();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Fill(1), Constraint::Length(28)])
        .split(area);

    let brand = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("><(((°>", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("CRUSH", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(concat!("v", env!("CARGO_PKG_VERSION")), Style::default().fg(DIM)),
        Span::raw("  "),
        Span::styled("container runtime", Style::default().fg(DIM).add_modifier(Modifier::ITALIC)),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(" crush ", Style::default().fg(DIM_ORANGE))));
    f.render_widget(brand, chunks[0]);

    let summary = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("●", Style::default().fg(RUN_GREEN)),
        Span::raw(" "),
        Span::styled(format!("{}", running), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" running  /  ", Style::default().fg(DIM)),
        Span::styled(format!("{}", app.containers.len()), Style::default().fg(Color::White)),
        Span::styled(" total  ", Style::default().fg(DIM)),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER)));
    f.render_widget(summary, chunks[1]);
}

fn render_tabs(f: &mut Frame, area: Rect, app: &mut App) {
    let titles: Vec<Line> = ["  Containers ", "  Stats ", "  Compose ", "  Logs ", "  Debug "]
        .iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .select(app.active_tab)
        .style(Style::default().fg(DIM))
        .highlight_style(Style::default().fg(ORANGE).add_modifier(Modifier::BOLD))
        .divider("│")
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER)));
    f.render_widget(tabs, area);
}

fn render_containers(f: &mut Frame, area: Rect, app: &mut App) {
    let header = Row::new(
        ["  ID", "NAME", "IMAGE", "STATUS", "CPU", "MEM", "PORTS", "UP"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)))
    )
    .height(1)
    .style(Style::default().bg(Color::Rgb(28, 24, 38)));

    let rows: Vec<Row> = app.containers.iter().enumerate().map(|(i, c)| {
        let selected = app.table_state.selected() == Some(i);
        let row_style = if selected { Style::default().bg(SEL_BG) } else { Style::default() };

        let (sym, sym_style) = match c.status {
            ContainerStatus::Running               => ("● Running",  Style::default().fg(RUN_GREEN)),
            ContainerStatus::Paused                => ("⏸ Paused",   Style::default().fg(PAUSE_YELLOW)),
            ContainerStatus::Stopped               => ("○ Stopped",  Style::default().fg(STOP_RED)),
            ContainerStatus::Creating |
            ContainerStatus::Created               => ("◌ Creating", Style::default().fg(CYAN)),
        };

        let short_id = if c.id.len() > 12 { &c.id[..12] } else { &c.id };
        let active = matches!(c.status, ContainerStatus::Running);

        let cpu_now = app.cpu_history.get(&c.id).and_then(|h| h.back().copied()).unwrap_or(0.0);
        let mem_now = app.mem_history.get(&c.id).and_then(|h| h.back().copied()).unwrap_or(0.0);

        let cpu_str = if active { format!("{:.1}%", cpu_now) } else { "--".into() };
        let mem_str = if active { format!("{:.0}M",  mem_now) } else { "--".into() };
        let cpu_color = if cpu_now > 80.0 { STOP_RED } else if cpu_now > 50.0 { PAUSE_YELLOW } else { RUN_GREEN };

        let ports = if c.ports.is_empty() { "--".into() } else {
            c.ports.iter().map(|p| format!("{}:{}", p.host_port, p.container_port)).collect::<Vec<_>>().join(",")
        };
        let uptime = format_duration(c.created_at);
        let arrow = if selected { "▶ " } else { "  " };

        Row::new(vec![
            Cell::from(format!("{}{}", arrow, short_id))
                .style(if selected { Style::default().fg(ORANGE).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }),
            Cell::from(c.name.clone())
                .style(if selected { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { row_style }),
            Cell::from(c.image.clone()).style(Style::default().fg(DIM)),
            Cell::from(sym).style(sym_style),
            Cell::from(cpu_str).style(Style::default().fg(cpu_color)),
            Cell::from(mem_str).style(Style::default().fg(CYAN)),
            Cell::from(ports).style(Style::default().fg(DIM)),
            Cell::from(uptime).style(Style::default().fg(DIM)),
        ])
        .height(1)
        .style(row_style)
    }).collect();

    let widths = [
        Constraint::Length(15), // ID
        Constraint::Length(16), // NAME
        Constraint::Fill(1),    // IMAGE
        Constraint::Length(12), // STATUS
        Constraint::Length(8),  // CPU
        Constraint::Length(7),  // MEM
        Constraint::Length(14), // PORTS
        Constraint::Length(6),  // UP
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_HOT))
        .title(Span::styled(
            " ><(((°>  containers ",
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ));

    if app.containers.is_empty() {
        let p = Paragraph::new("\n  No containers.  Run `crush run <image>` to start one.")
            .style(Style::default().fg(DIM))
            .block(block);
        f.render_widget(p, area);
    } else {
        let table = Table::new(rows, widths)
            .header(header)
            .block(block)
            .row_highlight_style(Style::default().bg(SEL_BG))
            .column_spacing(1);
        f.render_stateful_widget(table, area, &mut app.table_state);
    }
}

fn render_stats(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(area);

    let sel = app.selected().cloned();
    let left_title = sel.as_ref().map_or(
        " select a container  ↑↓ ".to_string(),
        |c| format!(" ><(((°>  {} ", c.name),
    );
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_HOT))
        .title(Span::styled(left_title, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)));
    let left_inner = left_block.inner(chunks[0]);
    f.render_widget(left_block, chunks[0]);

    if let Some(c) = &sel {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // id line
                Constraint::Length(1), // gap
                Constraint::Length(3), // cpu sparkline
                Constraint::Length(3), // mem sparkline
                Constraint::Length(3), // net in sparkline
                Constraint::Length(3), // net out sparkline
                Constraint::Length(3), // disk r sparkline
                Constraint::Length(3), // disk w sparkline
                Constraint::Fill(1),
            ])
            .split(left_inner);

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("id ", Style::default().fg(DIM)),
                Span::styled(c.id.clone(), Style::default().fg(Color::White)),
                Span::raw("   "),
                Span::styled("image ", Style::default().fg(DIM)),
                Span::styled(c.image.clone(), Style::default().fg(CYAN)),
            ])),
            rows[0],
        );

        let cpu_data: Vec<u64> = app.cpu_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let cpu_now = cpu_data.last().copied().unwrap_or(0);
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(ORANGE))
                    .title(Span::styled(format!(" CPU  {}% ", cpu_now), Style::default().fg(ORANGE).add_modifier(Modifier::BOLD))))
                .data(&cpu_data)
                .style(Style::default().fg(ORANGE)),
            rows[2],
        );

        let mem_data: Vec<u64> = app.mem_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let mem_now = mem_data.last().copied().unwrap_or(0);
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(CYAN))
                    .title(Span::styled(format!(" MEM  {} MB ", mem_now), Style::default().fg(CYAN).add_modifier(Modifier::BOLD))))
                .data(&mem_data)
                .style(Style::default().fg(CYAN)),
            rows[3],
        );

        let net_rx_data: Vec<u64> = app.net_rx_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let rx_now = app.net_rx_history.get(&c.id).and_then(|h| h.back().copied()).unwrap_or(0.0);
        let rx_color = if rx_now > 10_000_000.0 { STOP_RED } else if rx_now > 1_000_000.0 { PAUSE_YELLOW } else { RUN_GREEN };
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(rx_color))
                    .title(Span::styled(format!(" NET IN  {} ", format_rate(rx_now)), Style::default().fg(rx_color).add_modifier(Modifier::BOLD))))
                .data(&net_rx_data)
                .style(Style::default().fg(rx_color)),
            rows[4],
        );

        let net_tx_data: Vec<u64> = app.net_tx_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let tx_now = app.net_tx_history.get(&c.id).and_then(|h| h.back().copied()).unwrap_or(0.0);
        let tx_color = if tx_now > 10_000_000.0 { STOP_RED } else if tx_now > 1_000_000.0 { PAUSE_YELLOW } else { RUN_GREEN };
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(tx_color))
                    .title(Span::styled(format!(" NET OUT  {} ", format_rate(tx_now)), Style::default().fg(tx_color).add_modifier(Modifier::BOLD))))
                .data(&net_tx_data)
                .style(Style::default().fg(tx_color)),
            rows[5],
        );

        let disk_r_data: Vec<u64> = app.disk_r_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let dr_now = app.disk_r_history.get(&c.id).and_then(|h| h.back().copied()).unwrap_or(0.0);
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(DIM))
                    .title(Span::styled(format!(" DISK R  {} ", format_rate(dr_now)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))))
                .data(&disk_r_data)
                .style(Style::default().fg(Color::White)),
            rows[6],
        );

        let disk_w_data: Vec<u64> = app.disk_w_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let dw_now = app.disk_w_history.get(&c.id).and_then(|h| h.back().copied()).unwrap_or(0.0);
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(DIM))
                    .title(Span::styled(format!(" DISK W  {} ", format_rate(dw_now)), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))))
                .data(&disk_w_data)
                .style(Style::default().fg(Color::White)),
            rows[7],
        );
    }

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(" all ", Style::default().fg(DIM)));
    let right_inner = right_block.inner(chunks[1]);
    f.render_widget(right_block, chunks[1]);

    let per = 3u16;
    for (i, c) in app.containers.iter().enumerate() {
        let y = right_inner.y + i as u16 * per;
        if y + per > right_inner.y + right_inner.height { break; }
        let row_area = Rect { x: right_inner.x, y, width: right_inner.width, height: per };

        let cpu_data: Vec<u64> = app.cpu_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let cpu_now = cpu_data.last().copied().unwrap_or(0);
        let is_sel = app.table_state.selected() == Some(i);

        f.render_widget(
            Sparkline::default()
                .block(Block::default().title(Line::from(vec![
                    Span::styled(
                        format!("{} ", c.name),
                        Style::default().fg(if is_sel { Color::White } else { DIM }).add_modifier(if is_sel { Modifier::BOLD } else { Modifier::empty() }),
                    ),
                    Span::styled(format!("{}%", cpu_now), Style::default().fg(if is_sel { ORANGE } else { DIM_ORANGE })),
                ])))
                .data(&cpu_data)
                .style(Style::default().fg(if is_sel { ORANGE } else { DIM_ORANGE })),
            row_area,
        );
    }
}

fn render_compose(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Fill(1)])
        .split(area);

    let mut rows = Vec::new();
    for svc in &app.compose_services {
        let sym = if svc.status == "Running" { "● Running" } else { "○ Stopped" };
        let sym_style = if svc.status == "Running" { Style::default().fg(RUN_GREEN) } else { Style::default().fg(STOP_RED) };
        rows.push(Row::new(vec![
            Cell::from(svc.name.clone()).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(svc.image.clone()).style(Style::default().fg(DIM)),
            Cell::from(sym).style(sym_style),
        ]));
    }

    let header = Row::new(vec!["SERVICE", "IMAGE", "STATUS"])
        .style(Style::default().fg(ORANGE).add_modifier(Modifier::BOLD));
    let widths = [Constraint::Length(15), Constraint::Length(25), Constraint::Fill(1)];

    let grid = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER)).title(" compose services "));
    f.render_widget(grid, chunks[0]);

    // Service logs interleaved view
    let logs_p = Paragraph::new(app.compose_logs.join("\n"))
        .style(Style::default().fg(Color::Rgb(200, 200, 200)))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_HOT)).title(" services log stream (interleaved) "));
    f.render_widget(logs_p, chunks[1]);
}

fn render_logs(f: &mut Frame, area: Rect, app: &mut App) {
    let sel = app.selected().cloned();
    let title = sel.as_ref().map_or(
        " select a container  ↑↓ ".to_string(),
        |c| format!(" logs: {} ", c.name),
    );

    let container_id = sel.as_ref().map(|c| c.id.as_str()).unwrap_or("");
    let mut log_lines = if !container_id.is_empty() {
        app.get_streaming_logs(container_id)
    } else {
        vec![]
    };

    if app.searching && !app.search_query.is_empty() {
        let mut highlighted = Vec::new();
        for line in log_lines {
            if line.to_lowercase().contains(&app.search_query.to_lowercase()) {
                highlighted.push(format!(">>> {}", line));
            } else {
                highlighted.push(line);
            }
        }
        log_lines = highlighted;
    }

    let joined = log_lines.join("\n");
    let p = Paragraph::new(joined)
        .style(Style::default().fg(Color::Rgb(220, 220, 220)))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_HOT)).title(title));
    f.render_widget(p, area);
}

fn render_debug(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let (source_text, diag_text) = match &app.debug_info {
        Some((src, diag)) => (src.as_str(), diag.as_str()),
        None => (
            "  No container selected.\n  Navigate to Containers tab, select one, then press d.",
            "  Run `crush debug <id>` to load an AI diagnosis for a container.",
        ),
    };

    let left = Paragraph::new(source_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER)).title(" stderr / crash log "));
    f.render_widget(left, chunks[0]);

    let right = Paragraph::new(diag_text)
        .style(Style::default().fg(CYAN))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER_HOT)).title(" AI diagnosis & proposed patch "));
    f.render_widget(right, chunks[1]);
}

fn render_footer(f: &mut Frame, area: Rect, tab: usize) {
    let keys = match tab {
        0 => "  ↑↓ / jk  navigate    s  stats    Enter  inspect    q  quit  ",
        1 => "  ↑↓ / jk  select    p  containers    q  quit  ",
        2 => "  c  compose view    q  quit  ",
        3 => "  /  search    f  follow logs    q  quit  ",
        4 => "  d  debug mode    q  quit  ",
        _ => "  q  quit  ",
    };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(keys, Style::default().fg(DIM)),
        Span::styled("  Tab ", Style::default().fg(ORANGE)),
        Span::styled("switch view", Style::default().fg(DIM)),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER)));
    f.render_widget(footer, area);
}

// ─── Event loop ───────────────────────────────────────────────────────────────

fn run_interactive(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;

    let tick = Duration::from_millis(1000); // Live update at 1Hz
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // header
                    Constraint::Length(3), // tabs
                    Constraint::Fill(1),   // content
                    Constraint::Length(3), // footer
                ])
                .split(area);

            render_header(f, chunks[0], &mut app);
            render_tabs(f, chunks[1], &mut app);
            match app.active_tab {
                0 => render_containers(f, chunks[2], &mut app),
                1 => render_stats(f, chunks[2], &mut app),
                2 => render_compose(f, chunks[2], &mut app),
                3 => render_logs(f, chunks[2], &mut app),
                _ => render_debug(f, chunks[2], &mut app),
            }
            render_footer(f, chunks[3], app.active_tab);
        })?;

        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if app.searching {
                    match key.code {
                        KeyCode::Enter => { app.searching = false; }
                        KeyCode::Esc => { app.searching = false; app.search_query.clear(); }
                        KeyCode::Char(c) => { app.search_query.push(c); }
                        KeyCode::Backspace => { app.search_query.pop(); }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Down  | KeyCode::Char('j') => app.next_row(),
                        KeyCode::Up    | KeyCode::Char('k') => app.prev_row(),
                        KeyCode::Char('s') => app.active_tab = 1,
                        KeyCode::Char('p') => app.active_tab = 0,
                        KeyCode::Char('c') => app.active_tab = 2,
                        KeyCode::Char('l') => app.active_tab = 3,
                        KeyCode::Char('d') => app.active_tab = 4,
                        KeyCode::Char('/') => { app.searching = true; app.search_query.clear(); }
                        KeyCode::Char('f') => { app.follow = !app.follow; }
                        KeyCode::Tab       => app.active_tab = (app.active_tab + 1) % 5,
                        KeyCode::BackTab   => app.active_tab = (app.active_tab + 4) % 5,
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ─── Public TuiApp ────────────────────────────────────────────────────────────

pub struct TuiApp {
    _tick_rate: u64,
    data_dir: std::path::PathBuf,
}

impl TuiApp {
    pub fn new(tick_rate: u64, data_dir: std::path::PathBuf) -> Self {
        Self { _tick_rate: tick_rate, data_dir }
    }

    pub fn run_ps(&self, containers: Vec<Container>) -> io::Result<()> {
        run_interactive(App::new(containers, 0, self.data_dir.clone(), None, None))
    }

    pub fn run_stats(&self, containers: Vec<Container>) -> io::Result<()> {
        run_interactive(App::new(containers, 1, self.data_dir.clone(), None, None))
    }

    /// Open the TUI pre-focused on the Logs tab for a specific container.
    /// Passes through all containers so the user can still navigate away.
    pub fn run_logs(&self, containers: Vec<Container>, focused_id: &str) -> io::Result<()> {
        run_interactive(App::new(
            containers,
            3,
            self.data_dir.clone(),
            None,
            Some(focused_id.to_string()),
        ))
    }

    /// Open the TUI pre-focused on the Debug tab, pre-loaded with real diagnosis
    /// text produced by `crush debug`. `stderr_snippet` is the raw log/crash text;
    /// `diagnosis` is the formatted AI output (or offline pattern match).
    pub fn run_debug(
        &self,
        containers: Vec<Container>,
        focused_id: &str,
        stderr_snippet: String,
        diagnosis: String,
    ) -> io::Result<()> {
        run_interactive(App::new(
            containers,
            4,
            self.data_dir.clone(),
            Some((stderr_snippet, diagnosis)),
            Some(focused_id.to_string()),
        ))
    }

    // Non-interactive fallback for piped / non-TTY output
    fn format_status(c: &Container) -> String {
        match c.status {
            ContainerStatus::Running => {
                if let Some(started) = c.started_at {
                    let secs = started.elapsed().unwrap_or_default().as_secs();
                    format!("Up {}", Self::humanize_duration(secs))
                } else {
                    "Up".to_string()
                }
            }
            ContainerStatus::Stopped => "Exited".to_string(),
            ContainerStatus::Paused  => "Paused".to_string(),
            ContainerStatus::Created => "Created".to_string(),
            ContainerStatus::Creating => "Creating".to_string(),
        }
    }
    
    fn humanize_duration(secs: u64) -> String {
        if secs < 60 { format!("{} seconds", secs) }
        else if secs < 3600 { format!("{} minutes", secs / 60) }
        else if secs < 86400 { format!("{} hours", secs / 3600) }
        else { format!("{} days", secs / 86400) }
    }

    // Non-interactive fallback for piped / non-TTY output
    pub fn draw_containers_table(&self, containers: &[Container]) {
        println!("\x1b[1m\x1b[38;5;208m  ><(((°>  CRUSH  container runtime\x1b[0m");
        if containers.is_empty() {
            println!("  No containers running — run `crush run <image>` to start one");
            return;
        }

        let name_w = containers.iter().map(|c| c.name.len()).max().unwrap_or(4).max(4).min(40);
        let image_w = containers.iter().map(|c| c.image.len()).max().unwrap_or(5).max(5).min(40);
        let status_w = containers.iter().map(|c| Self::format_status(c).len()).max().unwrap_or(10).max(10).min(30);

        // Print header
        println!(
            "  \x1b[1m{:<12}  {:<name_w$}  {:<image_w$}  {:<status_w$}  METRICS\x1b[0m",
            "CONTAINER ID", "NAME", "IMAGE", "STATUS",
            name_w = name_w, image_w = image_w, status_w = status_w
        );

        for c in containers {
            let id = if c.id.len() > 12 { &c.id[..12] } else { &c.id };
            let name = if c.name.len() > name_w { &c.name[..name_w] } else { &c.name };
            let image = if c.image.len() > image_w { &c.image[..image_w] } else { &c.image };
            let status = Self::format_status(c);

            let status_styled = match c.status {
                ContainerStatus::Running => format!("\x1b[32m{}\x1b[0m", status),
                ContainerStatus::Paused => format!("\x1b[33m{}\x1b[0m", status),
                ContainerStatus::Stopped => format!("\x1b[31m{}\x1b[0m", status),
                _ => format!("\x1b[34m{}\x1b[0m", status),
            };

            // Calculate length of unstyled status for formatting alignment
            let status_len = status.len();

            println!(
                "  {:<12}  {:<name_w$}  {:<image_w$}  {}{}  --%/--M",
                id, name, image, status_styled, " ".repeat(status_w - status_len),
                name_w = name_w, image_w = image_w
            );
        }
    }

    pub fn draw_sparklines_graph(&self, name: &str, cpu_history: &[f32], mem_history: &[f32]) {
        let cq: VecDeque<f32> = cpu_history.iter().copied().collect();
        let mq: VecDeque<f32> = mem_history.iter().copied().collect();
        let cs = SparklineRenderer::render_dots(&cq, 30);
        let ms = SparklineRenderer::render_dots(&mq, 30);
        println!("\x1b[1m\x1b[38;5;208m  ><(((°>  CRUSH METRICS  {}\x1b[0m", name);
        println!("\x1b[38;5;240m  ┌──────────────────────────────────────────────────────────┐\x1b[0m");
        println!("  │ \x1b[38;5;208mCPU:\x1b[0m  [{:<30}]  \x1b[32m{:>5.1}%\x1b[0m   │", cs, cpu_history.last().unwrap_or(&0.0));
        println!("  │ \x1b[38;5;33mMEM:\x1b[0m  [{:<30}]  \x1b[32m{:>5.1} MB\x1b[0m │", ms, mem_history.last().unwrap_or(&0.0));
        println!("\x1b[38;5;240m  └──────────────────────────────────────────────────────────┘\x1b[0m");
    }
}
