use super::{CpuMetrics, TelemetryProvider};
use std::fs;
use std::path::Path;

#[derive(Default)]
pub struct LinuxTelemetry {
    prev_idle: u64,
    prev_total: u64,
    metrics: CpuMetrics,
}

impl LinuxTelemetry {
    pub fn new() -> Self {
        let mut inst = Self::default();
        inst.update();
        inst
    }

    fn read_cpu_temp() -> Option<f32> {
        let hwmon_base = Path::new("/sys/class/hwmon");
        if hwmon_base.exists() {
            if let Ok(entries) = fs::read_dir(hwmon_base) {
                let mut dirs: Vec<_> = entries.filter_map(|e| e.ok().map(|d| d.path())).collect();
                
                // Prioritize known CPU driver names
                let preferred_drivers = ["coretemp", "k10temp", "zenpower", "cpu_thermal", "soc_thermal", "acpitz"];
                dirs.sort_by_key(|dir| {
                    let name_path = dir.join("name");
                    if let Ok(content) = fs::read_to_string(&name_path) {
                        let name = content.trim().to_lowercase();
                        if preferred_drivers.iter().any(|d| name.contains(d)) {
                            return 0;
                        }
                    }
                    1
                });

                for dir in dirs {
                    if let Ok(files) = fs::read_dir(&dir) {
                        for file in files.filter_map(|f| f.ok()) {
                            let file_name = file.file_name().to_string_lossy().to_string();
                            if file_name.starts_with("temp") && file_name.ends_with("_input") {
                                let label_name = file_name.replace("_input", "_label");
                                let label_path = dir.join(label_name);
                                
                                if label_path.exists() {
                                    if let Ok(label_content) = fs::read_to_string(&label_path) {
                                        let label = label_content.trim().to_lowercase();
                                        if label.contains("tdie") || label.contains("tctl") || label.contains("package") || label.contains("cpu") {
                                            if let Ok(val_str) = fs::read_to_string(file.path()) {
                                                if let Ok(val) = val_str.trim().parse::<f32>() {
                                                    let c = if val > 1000.0 { val / 1000.0 } else { val };
                                                    if (10.0..=125.0).contains(&c) {
                                                        return Some(c);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Ok(val_str) = fs::read_to_string(file.path()) {
                                    if let Ok(val) = val_str.trim().parse::<f32>() {
                                        let c = if val > 1000.0 { val / 1000.0 } else { val };
                                        if (10.0..=125.0).contains(&c) {
                                            return Some(c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: /sys/class/thermal/thermal_zone*
        let thermal_base = Path::new("/sys/class/thermal");
        if thermal_base.exists() {
            if let Ok(entries) = fs::read_dir(thermal_base) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let temp_file = entry.path().join("temp");
                    if temp_file.exists() {
                        if let Ok(content) = fs::read_to_string(&temp_file) {
                            if let Ok(val) = content.trim().parse::<f32>() {
                                let c = if val > 1000.0 { val / 1000.0 } else { val };
                                if (10.0..=125.0).contains(&c) {
                                    return Some(c);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn read_cpu_load(&mut self) -> Option<f32> {
        if let Ok(content) = fs::read_to_string("/proc/stat") {
            if let Some(first_line) = content.lines().next() {
                if first_line.starts_with("cpu ") {
                    let parts: Vec<&str> = first_line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let user: u64 = parts[1].parse().unwrap_or(0);
                        let nice: u64 = parts[2].parse().unwrap_or(0);
                        let system: u64 = parts[3].parse().unwrap_or(0);
                        let idle: u64 = parts[4].parse().unwrap_or(0);
                        let iowait: u64 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
                        let irq: u64 = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
                        let softirq: u64 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
                        let steal: u64 = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);

                        let idle_all = idle + iowait;
                        let system_all = system + irq + softirq;
                        let total = user + nice + system_all + idle_all + steal;

                        let total_delta = total.saturating_sub(self.prev_total);
                        let idle_delta = idle_all.saturating_sub(self.prev_idle);

                        self.prev_total = total;
                        self.prev_idle = idle_all;

                        if total_delta > 0 {
                            let busy = total_delta.saturating_sub(idle_delta);
                            let load = (busy as f32 * 100.0) / (total_delta as f32);
                            return Some(load.clamp(0.0, 100.0));
                        }
                    }
                }
            }
        }
        None
    }

    fn read_cpu_freq() -> Option<f32> {
        if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
            let mut freqs = Vec::new();
            for line in content.lines() {
                if line.to_lowercase().starts_with("cpu mhz") {
                    if let Some(val_str) = line.split(':').nth(1) {
                        if let Ok(freq) = val_str.trim().parse::<f32>() {
                            freqs.push(freq);
                        }
                    }
                }
            }
            if !freqs.is_empty() {
                let avg = freqs.iter().sum::<f32>() / (freqs.len() as f32);
                return Some(avg.round());
            }
        }

        // Fallback: cpufreq scaling_cur_freq
        let cpufreq_base = Path::new("/sys/devices/system/cpu/cpufreq");
        if cpufreq_base.exists() {
            if let Ok(entries) = fs::read_dir(cpufreq_base) {
                let mut freqs = Vec::new();
                for entry in entries.filter_map(|e| e.ok()) {
                    let cur_freq_file = entry.path().join("scaling_cur_freq");
                    if cur_freq_file.exists() {
                        if let Ok(content) = fs::read_to_string(&cur_freq_file) {
                            if let Ok(khz) = content.trim().parse::<f32>() {
                                freqs.push(khz / 1000.0);
                            }
                        }
                    }
                }
                if !freqs.is_empty() {
                    let avg = freqs.iter().sum::<f32>() / (freqs.len() as f32);
                    return Some(avg.round());
                }
            }
        }

        None
    }
}

impl TelemetryProvider for LinuxTelemetry {
    fn update(&mut self) {
        self.metrics.temperature = Self::read_cpu_temp();
        self.metrics.load_percent = self.read_cpu_load();
        self.metrics.frequency_mhz = Self::read_cpu_freq();
    }

    fn get_metrics(&self) -> CpuMetrics {
        self.metrics.clone()
    }
}
