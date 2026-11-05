use super::{CpuMetrics, TelemetryProvider};
use std::fs;
use std::path::Path;

pub struct LinuxTelemetry {
    prev_idle: u64,
    prev_total: u64,
    metrics: CpuMetrics,
    temp_source: String,
}

impl Default for LinuxTelemetry {
    fn default() -> Self {
        Self {
            prev_idle: 0,
            prev_total: 0,
            metrics: CpuMetrics::default(),
            temp_source: "package".to_string(),
        }
    }
}

impl LinuxTelemetry {
    pub fn new() -> Self {
        let mut inst = Self::default();
        inst.update();
        inst
    }

    /// Read CPU temperature from Linux hwmon interface according to the selected source:
    /// "package" (Package / Tctl / Tdie), "core0" (First core), "avg" (Average cores), "max" (Max core).
    fn read_cpu_temp(&self) -> Option<f32> {
        let hwmon_base = Path::new("/sys/class/hwmon");
        if hwmon_base.exists() {
            if let Ok(entries) = fs::read_dir(hwmon_base) {
                let mut dirs: Vec<_> = entries.filter_map(|e| e.ok().map(|d| d.path())).collect();

                // Prioritize known CPU thermal drivers
                let preferred_drivers = [
                    "coretemp",
                    "k10temp",
                    "zenpower",
                    "cpu_thermal",
                    "soc_thermal",
                    "acpitz",
                ];
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

                let mut package_temp: Option<f32> = None;
                let mut core0_temp: Option<f32> = None;
                let mut core_temps: Vec<f32> = Vec::new();

                for dir in dirs {
                    if let Ok(files) = fs::read_dir(&dir) {
                        for file in files.filter_map(|f| f.ok()) {
                            let file_name = file.file_name().to_string_lossy().to_string();
                            if file_name.starts_with("temp") && file_name.ends_with("_input") {
                                let label_name = file_name.replace("_input", "_label");
                                let label_path = dir.join(label_name);

                                let label = if label_path.exists() {
                                    fs::read_to_string(&label_path)
                                        .unwrap_or_default()
                                        .trim()
                                        .to_lowercase()
                                } else {
                                    String::new()
                                };

                                if let Ok(val_str) = fs::read_to_string(file.path()) {
                                    if let Ok(val) = val_str.trim().parse::<f32>() {
                                        // sysfs temps are usually in millidegrees Celsius
                                        let c = if val > 1000.0 { val / 1000.0 } else { val };
                                        if (15.0..=120.0).contains(&c) {
                                            if label.contains("tdie")
                                                || label.contains("tctl")
                                                || label.contains("package")
                                            {
                                                if package_temp.is_none()
                                                    || c > package_temp.unwrap()
                                                {
                                                    package_temp = Some(c);
                                                }
                                            } else if label.contains("core 0")
                                                || label.contains("core0")
                                                || label.contains("cpu0")
                                            {
                                                if core0_temp.is_none() {
                                                    core0_temp = Some(c);
                                                }
                                                core_temps.push(c);
                                            } else if label.contains("core")
                                                || label.contains("cpu")
                                            {
                                                core_temps.push(c);
                                            } else {
                                                // Generic sensor fallback
                                                core_temps.push(c);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Resolve according to selected source
                let resolved = match self.temp_source.as_str() {
                    "core0" => core0_temp
                        .or_else(|| core_temps.first().copied())
                        .or(package_temp),
                    "avg" => {
                        if !core_temps.is_empty() {
                            let sum: f32 = core_temps.iter().sum();
                            Some((sum / core_temps.len() as f32).round())
                        } else {
                            package_temp
                        }
                    }
                    "max" => {
                        if !core_temps.is_empty() {
                            let m = core_temps.iter().cloned().fold(f32::MIN, f32::max);
                            Some(m.round())
                        } else {
                            package_temp
                        }
                    }
                    _ => {
                        // "package" or default
                        package_temp.or_else(|| {
                            if !core_temps.is_empty() {
                                let m = core_temps.iter().cloned().fold(f32::MIN, f32::max);
                                Some(m.round())
                            } else {
                                None
                            }
                        })
                    }
                };

                if let Some(t) = resolved {
                    return Some(t);
                }
            }
        }

        // Secondary Fallback: /sys/class/thermal/thermal_zone*
        let thermal_base = Path::new("/sys/class/thermal");
        if thermal_base.exists() {
            if let Ok(entries) = fs::read_dir(thermal_base) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let temp_file = entry.path().join("temp");
                    if temp_file.exists() {
                        if let Ok(content) = fs::read_to_string(&temp_file) {
                            if let Ok(val) = content.trim().parse::<f32>() {
                                let c = if val > 1000.0 { val / 1000.0 } else { val };
                                if (15.0..=120.0).contains(&c) {
                                    return Some(c.round());
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Read CPU load percentage from /proc/stat by calculating delta against previous sample.
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
}

impl TelemetryProvider for LinuxTelemetry {
    fn set_temp_source(&mut self, source: &str) {
        self.temp_source = source.to_string();
    }

    fn update(&mut self) {
        self.metrics.temperature = self.read_cpu_temp();
        self.metrics.load_percent = self.read_cpu_load();
    }

    fn get_metrics(&self) -> CpuMetrics {
        self.metrics.clone()
    }
}
