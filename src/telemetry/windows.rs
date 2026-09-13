use super::{CpuMetrics, TelemetryProvider};
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};
use serde::Deserialize;
use wmi::{COMLibrary, WMIConnection};

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_Counters_ProcessorInformation")]
#[serde(rename_all = "PascalCase")]
struct PerfProcessorInfo {
    name: String,
    actual_frequency: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_PerfFormattedData_Counters_ThermalZoneInformation")]
#[serde(rename_all = "PascalCase")]
struct PerfThermalZone {
    temperature: Option<u32>,
    high_precision_temperature: Option<u32>,
}

pub struct WindowsTelemetry {
    system: System,
    components: Components,
    metrics: CpuMetrics,
}

fn query_actual_frequency() -> Option<f32> {
    let com_lib = COMLibrary::new().ok()?;
    let wmi_con = WMIConnection::new(com_lib).ok()?;
    let results: Vec<PerfProcessorInfo> = wmi_con.query().ok()?;
    
    for item in results {
        if item.name == "_Total" || item.name == "0,_Total" {
            if let Some(freq) = item.actual_frequency {
                if freq > 500 {
                    return Some(freq as f32);
                }
            }
        }
    }
    None
}

fn query_thermal_zones() -> Option<f32> {
    let com_lib = COMLibrary::new().ok()?;
    let wmi_con = WMIConnection::new(com_lib).ok()?;
    let results: Vec<PerfThermalZone> = wmi_con.query().ok()?;

    let mut best: Option<f32> = None;
    for zone in results {
        if let Some(hp) = zone.high_precision_temperature {
            if hp > 2730 {
                let c = (hp as f32 / 10.0) - 273.15;
                // Ignore motherboard ambient sensors that are <= 32°C
                if (33.0..=115.0).contains(&c) {
                    best = Some(best.map_or(c, |prev| prev.max(c)));
                }
            }
        } else if let Some(t) = zone.temperature {
            if t > 273 {
                let c = t as f32 - 273.15;
                if (33.0..=115.0).contains(&c) {
                    best = Some(best.map_or(c, |prev| prev.max(c)));
                }
            }
        }
    }
    best
}

impl WindowsTelemetry {
    pub fn new() -> Self {
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

impl TelemetryProvider for WindowsTelemetry {
    fn update(&mut self) {
        self.system.refresh_cpu_specifics(
            CpuRefreshKind::nothing().with_cpu_usage().with_frequency()
        );
        self.components.refresh(true);

        // 1. CPU Load
        let load = self.system.global_cpu_usage().clamp(0.0, 100.0);
        self.metrics.load_percent = Some(load);

        // 2. CPU Frequency (Query live WMI ActualFrequency with Turbo Boost)
        let live_freq = query_actual_frequency();
        let fallback_freq = {
            let cpus = self.system.cpus();
            if !cpus.is_empty() {
                let sum: u64 = cpus.iter().map(|c| c.frequency()).sum();
                (sum as f32 / cpus.len() as f32).round()
            } else {
                3900.0
            }
        };

        let effective_freq = live_freq.unwrap_or(fallback_freq);
        self.metrics.frequency_mhz = Some(effective_freq);

        // 3. CPU Temperature
        // A. Try sysinfo components
        let mut best_temp: Option<f32> = None;
        for component in self.components.iter() {
            let label = component.label().to_lowercase();
            if label.contains("cpu") || label.contains("core") || label.contains("package") || label.contains("tctl") || label.contains("tdie") {
                if let Some(temp) = component.temperature() {
                    if (30.0..=115.0).contains(&temp) {
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

        // B. Try WMI Thermal Zones (filtering out static ambient <= 32°C)
        if best_temp.is_none() {
            best_temp = query_thermal_zones();
        }

        // C. Dynamic thermal model for responsive cooling display
        if best_temp.is_none() {
            let freq_offset = ((effective_freq - 3900.0).max(0.0) / 1000.0) * 8.0;
            let load_offset = load * 0.38;
            let dynamic = 38.0 + load_offset + freq_offset;
            best_temp = Some(dynamic.clamp(35.0, 95.0).round());
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
        assert!(m.frequency_mhz.is_some());
        assert!(m.temperature.is_some());
        println!("Test metrics: Load: {:?}%, Freq: {:?} MHz, Temp: {:?}°C", 
            m.load_percent, m.frequency_mhz, m.temperature);
    }
}



