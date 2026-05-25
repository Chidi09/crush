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
