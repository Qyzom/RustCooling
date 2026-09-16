use super::{CpuMetrics, TelemetryProvider};
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

pub struct WindowsTelemetry {
    system: System,
    components: Components,
    metrics: CpuMetrics,
    temp_source: String,
}

impl WindowsTelemetry {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage().with_frequency()),
        );
        sys.refresh_cpu_usage();
        let comps = Components::new_with_refreshed_list();

        Self {
            system: sys,
            components: comps,
            metrics: CpuMetrics::default(),
            temp_source: "package".to_string(),
        }
    }
}

impl Default for WindowsTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryProvider for WindowsTelemetry {
    fn set_temp_source(&mut self, source: &str) {
        self.temp_source = source.to_string();
    }

    fn update(&mut self) {
        self.system
            .refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage().with_frequency());
        if self.components.is_empty() {
            self.components.refresh(true);
        } else {
            self.components.refresh(false);
        }

        // 1. Refresh OS CPU usage & frequency
        let load = self.system.global_cpu_usage().clamp(0.0, 100.0);
        let effective_freq = {
            let cpus = self.system.cpus();
            if !cpus.is_empty() {
                let sum: u64 = cpus.iter().map(|c| c.frequency()).sum();
                (sum as f32 / cpus.len() as f32).round()
            } else {
                3900.0
            }
        };

        // 2. Read physical hardware digital thermal sensors via Ring 0 driver (LibreHardwareMonitor)
        let phys_temps = super::driver::read_physical_temperatures();
        let mut package_temp: Option<f32> = None;
        let mut core0_temp: Option<f32> = None;
        let mut core_temps: Vec<f32> = Vec::new();

        if let Some(p) = phys_temps {
            package_temp = p.package;
            core0_temp = p.core0;
            core_temps = p.core_temps;
        } else {
            // Fallback to sysinfo components if exposed by ACPI (e.g. laptops)
            for component in self.components.iter() {
                let label = component.label().to_lowercase();
                if let Some(temp) = component.temperature() {
                    if (25.0..=115.0).contains(&temp) {
                        if label.contains("package") || label.contains("tctl") || label.contains("tdie")
                        {
                            if package_temp.is_none() || temp > package_temp.unwrap() {
                                package_temp = Some(temp);
                            }
                        } else if label.contains("core 0")
                            || label.contains("core #0")
                            || label.contains("cpu core #0")
                        {
                            core0_temp = Some(temp);
                            core_temps.push(temp);
                        } else if label.contains("core") || label.contains("cpu") {
                            core_temps.push(temp);
                        }
                    }
                }
            }
        }

        self.metrics.load_percent = Some(load);
        self.metrics.frequency_mhz = Some(effective_freq);

        let chosen_temp: Option<f32> = match self.temp_source.as_str() {
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
                        Some((m + 2.0).round())
                    } else {
                        None
                    }
                })
            }
        };

        self.metrics.temperature = chosen_temp.map(|t| t.round());
    }

    fn get_metrics(&self) -> CpuMetrics {
        self.metrics.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_windows_metrics() {
        let mut t = WindowsTelemetry::new();
        println!("CPUs count: {}", t.system.cpus().len());
        for (i, cpu) in t.system.cpus().iter().enumerate() {
            println!(
                "CPU {}: usage={:.1}%, freq={}MHz",
                i,
                cpu.cpu_usage(),
                cpu.frequency()
            );
        }
        for src in &["package", "core0", "avg", "max"] {
            t.set_temp_source(src);
            t.update();
            let m = t.get_metrics();
            println!(
                "Source {}: Temp={:?}°C, Load={:?}%, Freq={:?}MHz",
                src, m.temperature, m.load_percent, m.frequency_mhz
            );
        }
    }
}
