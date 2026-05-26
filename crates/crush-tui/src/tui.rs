use std::collections::{HashMap, VecDeque};
use std::io;
use std::time::{Duration, SystemTime};
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

struct App {
    containers: Vec<Container>,
    cpu_history: HashMap<String, VecDeque<f32>>,
    mem_history: HashMap<String, VecDeque<f32>>,
    table_state: TableState,
    active_tab: usize,
    tick: u64,
}

impl App {
    fn new(containers: Vec<Container>, initial_tab: usize) -> Self {
        let mut table_state = TableState::default();
        if !containers.is_empty() { table_state.select(Some(0)); }

        // Seed smooth metric curves so the UI isn't empty on first open
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

        Self { containers, cpu_history, mem_history, table_state, active_tab: initial_tab, tick: 0 }
    }

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

    fn on_tick(&mut self) {
        self.tick += 1;
        for (i, c) in self.containers.iter().enumerate() {
            let t = self.tick as f32 * 0.14;
            let cpu = ((t + i as f32 * 1.3).sin().abs() * 28.0 + (i as f32 + 1.0) * 6.0).min(99.0);
            let mem = ((t * 0.4 + i as f32 * 0.8).cos().abs() * 55.0 + (i as f32 + 1.0) * 55.0).min(512.0);

            let h = self.cpu_history.entry(c.id.clone()).or_default();
            h.push_back(cpu);
            if h.len() > 60 { h.pop_front(); }

            let m = self.mem_history.entry(c.id.clone()).or_default();
            m.push_back(mem);
            if m.len() > 60 { m.pop_front(); }
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

    // Shark + brand name
    let brand = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("><(((°>", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("CRUSH", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled("v0.1.0", Style::default().fg(DIM)),
        Span::raw("  "),
        Span::styled("container runtime", Style::default().fg(DIM).add_modifier(Modifier::ITALIC)),
    ]))
    .block(Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(" crush ", Style::default().fg(DIM_ORANGE))));
    f.render_widget(brand, chunks[0]);

    // Live summary pill
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
    let titles: Vec<Line> = ["  Containers ", "  Stats ", "  Debug "]
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

    // ── Left: focused container ──
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
                Constraint::Length(4), // cpu sparkline
                Constraint::Length(1), // gap
                Constraint::Length(4), // mem sparkline
                Constraint::Fill(1),
            ])
            .split(left_inner);

        // ID + image
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

        // CPU sparkline
        let cpu_data: Vec<u64> = app.cpu_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let cpu_now = cpu_data.last().copied().unwrap_or(0);
        let cpu_label = format!(" CPU  {}% ", cpu_now);
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(ORANGE))
                    .title(Span::styled(cpu_label, Style::default().fg(ORANGE).add_modifier(Modifier::BOLD))))
                .data(&cpu_data)
                .style(Style::default().fg(ORANGE)),
            rows[2],
        );

        // MEM sparkline
        let mem_data: Vec<u64> = app.mem_history.get(&c.id)
            .map(|h| h.iter().map(|&v| v as u64).collect())
            .unwrap_or_default();
        let mem_now = mem_data.last().copied().unwrap_or(0);
        let mem_label = format!(" MEM  {} MB ", mem_now);
        f.render_widget(
            Sparkline::default()
                .block(Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(CYAN))
                    .title(Span::styled(mem_label, Style::default().fg(CYAN).add_modifier(Modifier::BOLD))))
                .data(&mem_data)
                .style(Style::default().fg(CYAN)),
            rows[4],
        );
    }

    // ── Right: all containers mini list ──
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

fn render_footer(f: &mut Frame, area: Rect, tab: usize) {
    let keys = match tab {
        0 => "  ↑↓ / jk  navigate    s  stats    Enter  inspect    q  quit  ",
        1 => "  ↑↓ / jk  select    p  containers    q  quit  ",
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

    let tick = Duration::from_millis(250);
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
                _ => f.render_widget(
                    Paragraph::new("\n  Debug view — coming soon.\n  Wire ANTHROPIC_API_KEY for AI diagnosis.")
                        .style(Style::default().fg(DIM))
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER))),
                    chunks[2],
                ),
            }
            render_footer(f, chunks[3], app.active_tab);
        })?;

        let timeout = tick.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Down  | KeyCode::Char('j') => app.next_row(),
                    KeyCode::Up    | KeyCode::Char('k') => app.prev_row(),
                    KeyCode::Char('s') => app.active_tab = 1,
                    KeyCode::Char('p') => app.active_tab = 0,
                    KeyCode::Tab       => app.active_tab = (app.active_tab + 1) % 3,
                    KeyCode::BackTab   => app.active_tab = (app.active_tab + 2) % 3,
                    _ => {}
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
}

impl TuiApp {
    pub fn new(tick_rate: u64) -> Self {
        Self { _tick_rate: tick_rate }
    }

    pub fn run_ps(&self, containers: Vec<Container>) -> io::Result<()> {
        run_interactive(App::new(containers, 0))
    }

    pub fn run_stats(&self, containers: Vec<Container>) -> io::Result<()> {
        run_interactive(App::new(containers, 1))
    }

    // Non-interactive fallback for piped / non-TTY output
    pub fn draw_containers_table(&self, containers: &[Container]) {
        println!("\x1b[1m\x1b[38;5;208m  ><(((°>  CRUSH  container runtime\x1b[0m");
        println!("\x1b[38;5;240m  ┌──────────────┬──────────────────────┬─────────────┬──────────┬──────────────┐\x1b[0m");
        println!("\x1b[38;5;244m  │ ID           │ NAME                 │ IMAGE       │ STATUS   │ METRICS      │\x1b[0m");
        println!("\x1b[38;5;240m  ├──────────────┼──────────────────────┼─────────────┼──────────┼──────────────┤\x1b[0m");
        if containers.is_empty() {
            println!("  │ \x1b[31mNo containers\x1b[0m — run `crush run <image>` to start one                               │");
        } else {
            for c in containers {
                let id  = if c.id.len()    > 12 { &c.id[..12]    } else { &c.id    };
                let nm  = if c.name.len()  > 20 { &c.name[..20]  } else { &c.name  };
                let img = if c.image.len() > 11 { &c.image[..11] } else { &c.image };
                let st  = match c.status {
                    ContainerStatus::Running => "\x1b[32m● Running\x1b[0m ",
                    ContainerStatus::Paused  => "\x1b[33m⏸ Paused\x1b[0m  ",
                    ContainerStatus::Stopped => "\x1b[31m○ Stopped\x1b[0m ",
                    _                        => "\x1b[34m◌ Creating\x1b[0m",
                };
                println!("  │ \x1b[38;5;248m{:<12}\x1b[0m │ \x1b[1m{:<20}\x1b[0m │ {:<11} │ {:<18} │ --%/--M      │",
                    id, nm, img, st);
            }
        }
        println!("\x1b[38;5;240m  └──────────────┴──────────────────────┴─────────────┴──────────┴──────────────┘\x1b[0m");
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
