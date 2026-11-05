use super::{CpuMetrics, TelemetryProvider};
use std::fs;
use std::path::{Path, PathBuf};

pub struct LinuxTelemetry {
    prev_idle: u64,
    prev_total: u64,
    metrics: CpuMetrics,
    temp_source: String,
    cached_paths: Vec<PathBuf>,
}

impl Default for LinuxTelemetry {
    fn default() -> Self {
        Self {
            prev_idle: 0,
            prev_total: 0,
            metrics: CpuMetrics::default(),
            temp_source: "package".to_string(),
            cached_paths: Vec::new(),
        }
    }
}

impl LinuxTelemetry {
    pub fn new() -> Self {
        let mut inst = Self::default();
        inst.update();
        inst
    }

    fn read_sensor_file(path: &Path) -> Option<f32> {
        let val_str = fs::read_to_string(path).ok()?;
        let val = val_str.trim().parse::<f32>().ok()?;
        let c = if val > 1000.0 { val / 1000.0 } else { val };
        if (15.0..=120.0).contains(&c) {
            Some(c)
        } else {
            None
        }
    }

    /// Read CPU temperature from Linux hwmon interface according to the selected source:
    /// "package" (Package / Tctl / Tdie), "core0" (First core), "avg" (Average cores), "max" (Max core).
    fn read_cpu_temp(&mut self) -> Option<f32> {
        // Fast path: read from cached paths if available
        if !self.cached_paths.is_empty() {
            let mut readings: Vec<f32> = Vec::with_capacity(self.cached_paths.len());
            for path in &self.cached_paths {
                if let Some(c) = Self::read_sensor_file(path) {
                    readings.push(c);
                }
            }

            if !readings.is_empty() {
                return match self.temp_source.as_str() {
                    "core0" => readings.first().copied(),
                    "avg" => {
                        let sum: f32 = readings.iter().sum();
                        Some((sum / readings.len() as f32).round())
                    }
                    "max" => {
                        let m = readings.iter().cloned().fold(f32::MIN, f32::max);
                        Some(m.round())
                    }
                    _ => readings.first().copied(), // package is single sensor
                };
            }

            // If reading from cached paths failed, clear cache and re-discover
            self.cached_paths.clear();
        }

        // Slow path: scan hwmon devices and cache paths
        let hwmon_base = Path::new("/sys/class/hwmon");
        if hwmon_base.exists() {
            if let Ok(entries) = fs::read_dir(hwmon_base) {
                let mut hwmon_devices: Vec<(PathBuf, String)> = entries
                    .filter_map(|e| e.ok().map(|d| d.path()))
                    .map(|dir| {
                        let name = fs::read_to_string(dir.join("name"))
                            .ok()
                            .map(|s| s.trim().to_lowercase())
                            .unwrap_or_default();
                        (dir, name)
                    })
                    .collect();

                let preferred_cpu_drivers = [
                    "coretemp",
                    "k10temp",
                    "zenpower",
                    "cpu_thermal",
                    "soc_thermal",
                ];

                hwmon_devices.sort_by_key(|(_, name)| {
                    if preferred_cpu_drivers.iter().any(|d| name.contains(d)) {
                        0
                    } else if name.contains("acpitz") {
                        1
                    } else {
                        2
                    }
                });

                let mut package_path: Option<(PathBuf, f32)> = None;
                let mut core0_path: Option<(PathBuf, f32)> = None;
                let mut core_paths: Vec<(PathBuf, f32)> = Vec::new();
                let mut found_primary_cpu = false;

                for (dir, driver_name) in hwmon_devices {
                    let is_cpu_driver = preferred_cpu_drivers.iter().any(|d| driver_name.contains(d));

                    if found_primary_cpu && !is_cpu_driver {
                        continue;
                    }

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

                                if !is_cpu_driver {
                                    let is_likely_cpu = label.contains("cpu")
                                        || label.contains("package")
                                        || label.contains("core")
                                        || label.contains("tctl")
                                        || label.contains("tdie");
                                    if !is_likely_cpu {
                                        continue;
                                    }
                                }

                                if let Some(c) = Self::read_sensor_file(&file.path()) {
                                    if is_cpu_driver {
                                        found_primary_cpu = true;
                                    }

                                    if label.contains("tdie")
                                        || label.contains("tctl")
                                        || label.contains("package")
                                    {
                                        if package_path.is_none() || c > package_path.as_ref().unwrap().1 {
                                            package_path = Some((file.path(), c));
                                        }
                                    } else if label.contains("core 0")
                                        || label.contains("core0")
                                        || label.contains("cpu0")
                                    {
                                        if core0_path.is_none() {
                                            core0_path = Some((file.path(), c));
                                        }
                                        core_paths.push((file.path(), c));
                                    } else if label.contains("core") || label.contains("cpu") || is_cpu_driver {
                                        core_paths.push((file.path(), c));
                                    }
                                }
                            }
                        }
                    }
                }

                match self.temp_source.as_str() {
                    "core0" => {
                        if let Some((p, c)) = core0_path.or_else(|| core_paths.first().cloned()).or(package_path) {
                            self.cached_paths = vec![p];
                            return Some(c);
                        }
                    }
                    "avg" => {
                        if !core_paths.is_empty() {
                            let sum: f32 = core_paths.iter().map(|(_, c)| c).sum();
                            let avg = (sum / core_paths.len() as f32).round();
                            self.cached_paths = core_paths.into_iter().map(|(p, _)| p).collect();
                            return Some(avg);
                        } else if let Some((p, c)) = package_path {
                            self.cached_paths = vec![p];
                            return Some(c);
                        }
                    }
                    "max" => {
                        if !core_paths.is_empty() {
                            let max_val = core_paths.iter().map(|(_, c)| *c).fold(f32::MIN, f32::max).round();
                            self.cached_paths = core_paths.into_iter().map(|(p, _)| p).collect();
                            return Some(max_val);
                        } else if let Some((p, c)) = package_path {
                            self.cached_paths = vec![p];
                            return Some(c);
                        }
                    }
                    _ => {
                        // "package" or default
                        if let Some((p, c)) = package_path.or_else(|| core_paths.first().cloned()) {
                            self.cached_paths = vec![p];
                            return Some(c);
                        }
                    }
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
                        if let Some(c) = Self::read_sensor_file(&temp_file) {
                            self.cached_paths = vec![temp_file];
                            return Some(c.round());
                        }
                    }
                }
            }
        }

        None
    }

    /// Read CPU load percentage from /proc/stat by calculating delta against previous sample.
    fn read_cpu_load(&mut self) -> Option<f32> {
        let file = fs::File::open("/proc/stat").ok()?;
        let mut reader = std::io::BufReader::new(file);
        let mut first_line = String::new();
        use std::io::BufRead;
        reader.read_line(&mut first_line).ok()?;

        if first_line.starts_with("cpu ") {
            let mut parts = first_line.split_whitespace().skip(1);
            let user: u64 = parts.next()?.parse().unwrap_or(0);
            let nice: u64 = parts.next()?.parse().unwrap_or(0);
            let system: u64 = parts.next()?.parse().unwrap_or(0);
            let idle: u64 = parts.next()?.parse().unwrap_or(0);
            let iowait: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let irq: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let softirq: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let steal: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

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
        None
    }
}

impl TelemetryProvider for LinuxTelemetry {
    fn set_temp_source(&mut self, source: &str) {
        if self.temp_source != source {
            self.temp_source = source.to_string();
            self.cached_paths.clear();
        }
    }

    fn update(&mut self) {
        self.metrics.temperature = self.read_cpu_temp();
        self.metrics.load_percent = self.read_cpu_load();
    }

    fn get_metrics(&self) -> CpuMetrics {
        self.metrics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_telemetry_initialization() {
        let mut provider = LinuxTelemetry::new();
        provider.set_temp_source("package");
        provider.update();
        let metrics = provider.get_metrics();
        if let Some(t) = metrics.temperature {
            assert!((10.0..=125.0).contains(&t));
        }
    }
}
