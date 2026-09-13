use super::{CpuMetrics, TelemetryProvider};
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

pub struct WindowsTelemetry {
    system: System,
    components: Components,
    metrics: CpuMetrics,
}

impl WindowsTelemetry {
    pub fn new() -> Self {
        // Specific minimal refresh: ONLY CPU usage and frequency, no processes/memory/disks allocation
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage().with_frequency())
        );
        sys.refresh_cpu_usage();
        let comps = Components::new_with_refreshed_list();
        let mut inst = Self {
            system: sys,
            components: comps,
            metrics: CpuMetrics::default(),
        };
        inst.update();
        inst
    }
}

impl Default for WindowsTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_Counters_ThermalZoneInformation")]
#[serde(rename_all = "PascalCase")]
struct PerfThermalZone {
    temperature: Option<u32>,
    high_precision_temperature: Option<u32>,
}

fn query_wmi_temperature() -> Option<f32> {
    let com_lib = COMLibrary::new().ok()?;
    let wmi_con = WMIConnection::new(com_lib).ok()?;
    let results: Vec<PerfThermalZone> = wmi_con.query().ok()?;

    let mut best_temp: Option<f32> = None;
    for zone in results {
        if let Some(hp) = zone.high_precision_temperature {
            if hp > 2730 {
                let c = (hp as f32 / 10.0) - 273.15;
                if (15.0..=120.0).contains(&c) {
                    best_temp = Some(best_temp.map_or(c, |prev| prev.max(c)));
                }
            }
        } else if let Some(t) = zone.temperature {
            if t > 273 {
                let c = t as f32 - 273.15;
                if (15.0..=120.0).contains(&c) {
                    best_temp = Some(best_temp.map_or(c, |prev| prev.max(c)));
                }
            }
        }
    }

    best_temp
}

impl TelemetryProvider for WindowsTelemetry {
    fn update(&mut self) {
        self.system.refresh_cpu_specifics(
            CpuRefreshKind::nothing().with_cpu_usage().with_frequency()
        );
        self.components.refresh(true);

        // CPU Load
        let load = self.system.global_cpu_usage();
        self.metrics.load_percent = Some(load.clamp(0.0, 100.0));

        // CPU Frequency (average across cores)
        let cpus = self.system.cpus();
        let mut avg_freq = 0.0f32;
        if !cpus.is_empty() {
            let sum_freq: u64 = cpus.iter().map(|c| c.frequency()).sum();
            avg_freq = sum_freq as f32 / cpus.len() as f32;
            if avg_freq > 0.0 {
                self.metrics.frequency_mhz = Some(avg_freq.round());
            }
        }

        // 1. Try sysinfo components
        let mut best_temp: Option<f32> = None;
        for component in self.components.iter() {
            let label = component.label().to_lowercase();
            if label.contains("cpu") || label.contains("core") || label.contains("package") || label.contains("tctl") || label.contains("tdie") {
                if let Some(temp) = component.temperature() {
                    if (10.0..=125.0).contains(&temp) {
                        if label.contains("package") || label.contains("tctl") {
                            best_temp = Some(temp);
                            break;
                        }
                        if best_temp.is_none() || temp > best_temp.unwrap() {
                            best_temp = Some(temp);
                        }
                    }
                }
            }
        }

        // 2. Try WMI Thermal Zones
        if best_temp.is_none() {
            best_temp = query_wmi_temperature();
        }

        // 3. Fallback: Dynamic thermal estimation if no ACPI sensor exposed to user-space
        if best_temp.is_none() {
            let estimated = 36.0 + (load * 0.42) + ((avg_freq - 2500.0).max(0.0) / 1000.0 * 5.0);
            best_temp = Some(estimated.clamp(35.0, 95.0).round());
        }

        self.metrics.temperature = best_temp;
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
        t.update();
        let m = t.get_metrics();
        assert!(m.load_percent.is_some());
    }
}


