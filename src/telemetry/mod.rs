#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub mod driver;

#[derive(Debug, Clone, Default)]
pub struct CpuMetrics {
    pub temperature: Option<f32>,
    pub load_percent: Option<f32>,
}

pub trait TelemetryProvider: Send + Sync {
    fn update(&mut self);
    fn get_metrics(&self) -> CpuMetrics;
    fn set_temp_source(&mut self, _source: &str) {}
}

pub fn create_telemetry_provider() -> Box<dyn TelemetryProvider> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxTelemetry::new())
    }
    #[cfg(windows)]
    {
        Box::new(windows::WindowsTelemetry::new())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        struct Dummy;
        impl TelemetryProvider for Dummy {
            fn update(&mut self) {}
            fn get_metrics(&self) -> CpuMetrics {
                CpuMetrics::default()
            }
        }
        Box::new(Dummy)
    }
}
