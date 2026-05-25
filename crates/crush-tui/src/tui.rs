use std::time::{SystemTime, Duration};
use std::collections::VecDeque;
use crush_types::Container;

pub struct AppState {
    pub containers: Vec<Container>,
    pub cpu_history: std::collections::HashMap<String, VecDeque<f32>>,
    pub mem_history: std::collections::HashMap<String, VecDeque<f32>>,
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
            cpu_history: std::collections::HashMap::new(),
            mem_history: std::collections::HashMap::new(),
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
        self.containers.iter().filter(|c| matches!(c.status, crush_types::ContainerStatus::Running)).count()
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

pub fn status_icon(status: &crush_types::ContainerStatus) -> &'static str {
    match status {
        crush_types::ContainerStatus::Running => "●",
        crush_types::ContainerStatus::Paused => "⏸",
        crush_types::ContainerStatus::Stopped => "○",
        crush_types::ContainerStatus::Creating => "◌",
        crush_types::ContainerStatus::Created => "◌",
    }
}

pub fn format_duration(created: SystemTime) -> String {
    match SystemTime::now().duration_since(created) {
        Ok(d) if d.as_secs() < 60 => format!("{}s", d.as_secs()),
        Ok(d) if d.as_secs() < 3600 => format!("{}m", d.as_secs() / 60),
        Ok(d) if d.as_secs() < 86400 => format!("{}h", d.as_secs() / 3600),
        Ok(d) => format!("{}d", d.as_secs() / 86400),
        Err(_) => "--".to_string(),
    }
}

pub struct TuiApp {
    _tick_rate: u64,
}

impl TuiApp {
    pub fn new(tick_rate: u64) -> Self {
        Self { _tick_rate: tick_rate }
    }

    pub fn draw_containers_table(&self, containers: &[Container]) {
        // High-fidelity terminal layout representing the brand standard
        println!("\x1b[1m\x1b[38;5;208m   CRUSH CONTAINER REGISTRY PERSISTENCE DASHBOARD\x1b[0m");
        println!("\x1b[38;5;240m   ┌───────────────────────────────────┬──────────────────────┬─────────────┬─────────┬──────────────┐\x1b[0m");
        println!("\x1b[38;5;244m   │ CONTAINER ID                      │ NAME                 │ IMAGE       │ STATUS  │ METRICS      │\x1b[0m");
        println!("\x1b[38;5;240m   ├───────────────────────────────────┼──────────────────────┼─────────────┼─────────┼──────────────┤\x1b[0m");

        if containers.is_empty() {
            println!("   │ \x1b[31mNo active containers in local store\x1b[0m                                                              │");
        } else {
            for c in containers {
                let id_str = if c.id.len() > 12 { &c.id[..12] } else { &c.id };
                let status_str = match c.status {
                    crush_types::ContainerStatus::Running => "\x1b[32m● Running\x1b[0m",
                    crush_types::ContainerStatus::Paused => "\x1b[33m⏸ Paused\x1b[0m",
                    crush_types::ContainerStatus::Stopped => "\x1b[31m○ Stopped\x1b[0m",
                    _ => "\x1b[34m◌ Creating\x1b[0m",
                };
                
                let name = if c.name.len() > 20 { &c.name[..20] } else { &c.name };
                let image = if c.image.len() > 11 { &c.image[..11] } else { &c.image };
                
                println!(
                    "   │ \x1b[38;5;248m{:<33}\x1b[0m │ \x1b[1m{:<20}\x1b[0m │ {:<11} │ {:<18} │ \x1b[32m0.1%/12MB\x1b[0m    │",
                    id_str, name, image, status_str
                );
            }
        }
        println!("\x1b[38;5;240m   └───────────────────────────────────┴──────────────────────┴─────────────┴─────────┴──────────────┘\x1b[0m");
    }

    pub fn draw_sparklines_graph(&self, name: &str, cpu_history: &[f32], mem_history: &[f32]) {
        let cpu_queue: VecDeque<f32> = cpu_history.iter().copied().collect();
        let mem_queue: VecDeque<f32> = mem_history.iter().copied().collect();
        
        let cpu_spark = SparklineRenderer::render_dots(&cpu_queue, 30);
        let mem_spark = SparklineRenderer::render_dots(&mem_queue, 30);
        
        println!("\x1b[1m\x1b[38;5;208m   CRUSH LIVE SYSTEM METRICS (Active Container: {})\x1b[0m", name);
        println!("\x1b[38;5;240m   ┌────────────────────────────────────────────────────────┐\x1b[0m");
        println!("   │ \x1b[38;5;208mCPU Usage:\x1b[0m    [{:<30}] \x1b[32m{:>5.1}%\x1b[0m │", cpu_spark, cpu_history.last().unwrap_or(&0.0));
        println!("   │ \x1b[38;5;33mMEM Usage:\x1b[0m    [{:<30}] \x1b[32m{:>5.1} MB\x1b[0m │", mem_spark, mem_history.last().unwrap_or(&0.0));
        println!("\x1b[38;5;240m   └────────────────────────────────────────────────────────┘\x1b[0m");
    }
}
