use crate::pipeline::PipelineResult;

pub struct BuildProgress {
    phase_times: Vec<PhaseTime>,
}

struct PhaseTime {
    name: String,
    elapsed_ms: u64,
    _size_bytes: u64,
    cached: bool,
}

impl BuildProgress {
    pub fn new() -> Self {
        Self {
            phase_times: Vec::new(),
        }
    }

    pub fn record_phase(&mut self, name: &str, elapsed_ms: u64, size_bytes: u64, cached: bool) {
        self.phase_times.push(PhaseTime {
            name: name.to_string(),
            elapsed_ms,
            _size_bytes: size_bytes,
            cached,
        });
    }

    pub fn print_summary(&self, result: &PipelineResult) {
        println!("\n── Build Summary ───────────────────────");
        println!("  Total: {:.1}s", result.timing.total_ms as f64 / 1000.0);
        println!("  Digest: {}", &result.digest[..19]);

        for phase in &self.phase_times {
            let cache_mark = if phase.cached { " (cached)" } else { "" };
            println!("  ├─ {}: {:.1}s{}", phase.name, phase.elapsed_ms as f64 / 1000.0, cache_mark);
        }

        let total_size: u64 = result.layers.iter().map(|l| l.size_bytes).sum();
        let cached_count = result.layers.iter().filter(|l| l.cached).count();
        let uncached_count = result.layers.len() - cached_count;

        println!("  Layers: {} total, {} cached, {} new",
            result.layers.len(), cached_count, uncached_count);
        println!("  Size: {} MB", total_size as f64 / 1024.0 / 1024.0);

        if !result.layers.is_empty() {
            let prev = result.layers.first().map(|l| l.size_bytes).unwrap_or(0);
            let delta = if total_size > prev {
                format!("+{} MB", (total_size - prev) as f64 / 1024.0 / 1024.0)
            } else {
                format!("{} MB", total_size as f64 / 1024.0 / 1024.0)
            };
            println!("  Delta: {}", delta);
        }

        println!("────────────────────────────────────────\n");
    }
}
