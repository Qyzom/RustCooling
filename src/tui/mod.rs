use crate::config::AppConfig;
use crate::i18n::I18n;
use crate::service::MonitorService;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Tabs,
    },
    Frame, Terminal,
};
use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct TerminalCleanupGuard;

impl Drop for TerminalCleanupGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

pub struct TuiApp {
    config_ref: Arc<Mutex<AppConfig>>,
    monitor: Arc<MonitorService>,
    selected_tab: usize,
    selected_setting: usize,
    temp_history: Vec<u64>,
    load_history: Vec<u64>,
    status_message: Option<(String, Instant)>,

    display_mode: String,
    update_interval_ms: u64,
    temp_source: String,
    temp_smoothing: u32,
    language: String,
    custom_vid: u16,
    custom_pid: u16,
    autostart_mode: String,
    show_tray: bool,
}

impl TuiApp {
    pub fn new(config_ref: Arc<Mutex<AppConfig>>, monitor: Arc<MonitorService>) -> Self {
        let cfg = config_ref.lock().unwrap().clone();
        Self {
            config_ref,
            monitor,
            selected_tab: 0,
            selected_setting: 0,
            temp_history: Vec::with_capacity(60),
            load_history: Vec::with_capacity(60),
            status_message: None,

            display_mode: cfg.display_mode,
            update_interval_ms: cfg.update_interval_ms,
            temp_source: cfg.temp_source,
            temp_smoothing: cfg.temp_smoothing,
            language: cfg.language,
            custom_vid: cfg.custom_vid,
            custom_pid: cfg.custom_pid,
            autostart_mode: cfg.autostart_mode,
            show_tray: cfg.show_tray,
        }
    }

    /// Runs TUI loop. Returns `Ok(true)` if background daemon should be kept running,
    /// or `Ok(false)` if user explicitly requested full shutdown (power off screen).
    pub fn run(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let _guard = TerminalCleanupGuard;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let mut last_tick = Instant::now();
        let keep_daemon_running;

        loop {
            let state = self.monitor.get_state();
            if let Ok(m) = state.metrics.lock() {
                if let Some(t) = m.temperature {
                    if self.temp_history.len() >= 60 {
                        self.temp_history.remove(0);
                    }
                    self.temp_history.push(t.round().clamp(0.0, 120.0) as u64);
                }
                if let Some(l) = m.load_percent {
                    if self.load_history.len() >= 60 {
                        self.load_history.remove(0);
                    }
                    self.load_history.push(l.round().clamp(0.0, 100.0) as u64);
                }
            }

            terminal.draw(|f| self.ui(f))?;

            let timeout = Duration::from_millis(100).saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        keep_daemon_running = true;
                        break;
                    }

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            keep_daemon_running = true;
                            break;
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            keep_daemon_running = false;
                            break;
                        }
                        KeyCode::Tab => {
                            self.selected_tab = (self.selected_tab + 1) % 2;
                        }
                        KeyCode::BackTab => {
                            self.selected_tab = if self.selected_tab == 0 { 1 } else { 0 };
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            self.save_settings();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            self.reset_defaults();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.selected_tab == 1 && self.selected_setting > 0 {
                                self.selected_setting -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.selected_tab == 1 && self.selected_setting < 11 {
                                self.selected_setting += 1;
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('-') if self.selected_tab == 1 => {
                            self.adjust_setting(false);
                        }
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('+') | KeyCode::Enter | KeyCode::Char(' ') if self.selected_tab == 1 => {
                            if self.selected_setting == 11 {
                                keep_daemon_running = false;
                                break;
                            } else {
                                self.adjust_setting(true);
                            }
                        }
                        _ => {}
                    }
                }
            }

            if last_tick.elapsed() >= Duration::from_millis(100) {
                last_tick = Instant::now();
            }
        }

        Ok(keep_daemon_running)
    }

    fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    fn save_settings(&mut self) {
        I18n::set_language(&self.language);
        let save_res = {
            let mut cfg = self.config_ref.lock().unwrap();
            cfg.display_mode = self.display_mode.clone();
            cfg.update_interval_ms = self.update_interval_ms;
            cfg.temp_source = self.temp_source.clone();
            cfg.temp_smoothing = self.temp_smoothing;
            cfg.language = self.language.clone();
            cfg.custom_vid = self.custom_vid;
            cfg.custom_pid = self.custom_pid;
            cfg.autostart_mode = self.autostart_mode.clone();
            cfg.show_tray = self.show_tray;
            cfg.auto_start = self.autostart_mode != "none";
            cfg.save()
        };

        crate::set_autostart_mode(&self.autostart_mode);

        if let Err(e) = save_res {
            self.set_status(&format!("Error saving config: {}", e));
        } else {
            self.set_status("[✓] Settings saved successfully to config.json!");
        }
    }

    fn reset_defaults(&mut self) {
        let def = AppConfig::default();
        self.display_mode = def.display_mode.clone();
        self.update_interval_ms = def.update_interval_ms;
        self.temp_source = def.temp_source.clone();
        self.temp_smoothing = def.temp_smoothing;
        self.language = def.language.clone();
        self.custom_vid = def.custom_vid;
        self.custom_pid = def.custom_pid;
        self.autostart_mode = def.autostart_mode.clone();
        self.show_tray = def.show_tray;
        I18n::set_language(&self.language);
        crate::set_autostart_mode("none");
        if let Ok(mut cfg) = self.config_ref.lock() {
            *cfg = def.clone();
            let _ = cfg.save();
        }
        self.set_status("[✓] Settings reset to default values.");
    }

    fn adjust_setting(&mut self, forward: bool) {
        match self.selected_setting {
            0 => {
                self.display_mode = if self.display_mode == "temp" {
                    "load".to_string()
                } else {
                    "temp".to_string()
                };
            }
            1 => {
                if forward {
                    self.update_interval_ms = (self.update_interval_ms + 100).min(3000);
                } else {
                    self.update_interval_ms = self.update_interval_ms.saturating_sub(100).max(100);
                }
            }
            2 => {
                let sources = ["package", "core0", "avg", "max"];
                let pos = sources.iter().position(|&s| s == self.temp_source).unwrap_or(0);
                let new_pos = if forward {
                    (pos + 1) % sources.len()
                } else {
                    (pos + sources.len() - 1) % sources.len()
                };
                self.temp_source = sources[new_pos].to_string();
            }
            3 => {
                if forward {
                    self.temp_smoothing = (self.temp_smoothing + 1).min(5);
                } else {
                    self.temp_smoothing = self.temp_smoothing.saturating_sub(1);
                }
            }
            4 => {
                let langs = ["en", "ru", "zh", "de", "fr"];
                let pos = langs.iter().position(|&l| l == self.language).unwrap_or(0);
                let new_pos = if forward {
                    (pos + 1) % langs.len()
                } else {
                    (pos + langs.len() - 1) % langs.len()
                };
                self.language = langs[new_pos].to_string();
                I18n::set_language(&self.language);
            }
            5 => {
                if forward {
                    self.custom_vid = self.custom_vid.wrapping_add(1);
                } else {
                    self.custom_vid = self.custom_vid.wrapping_sub(1);
                }
            }
            6 => {
                if forward {
                    self.custom_pid = self.custom_pid.wrapping_add(1);
                } else {
                    self.custom_pid = self.custom_pid.wrapping_sub(1);
                }
            }
            7 => {
                let modes = ["none", "daemon", "gui"];
                let pos = modes.iter().position(|&m| m == self.autostart_mode).unwrap_or(0);
                let new_pos = if forward {
                    (pos + 1) % modes.len()
                } else {
                    (pos + modes.len() - 1) % modes.len()
                };
                self.autostart_mode = modes[new_pos].to_string();
            }
            8 => {
                self.show_tray = !self.show_tray;
            }
            9 => {
                self.save_settings();
            }
            10 => {
                self.reset_defaults();
            }
            _ => {}
        }
    }

    fn ui(&self, frame: &mut Frame) {
        let t = I18n::get();
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Header & Tabs
                Constraint::Min(12),   // Main Body
                Constraint::Length(3), // Footer Status
            ])
            .split(size);

        // Header Layout
        let header_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        let title_text = format!(" RustCooling v{} ", env!("CARGO_PKG_VERSION"));
        let title_widget = Paragraph::new(Line::from(vec![
            Span::styled(title_text, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" [ID-COOLING FX Controller]", Style::default().fg(Color::DarkGray)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
        frame.render_widget(title_widget, header_chunks[0]);

        let tab_titles = vec![
            format!(" 1. {} ", t.app_title),
            format!(" 2. {} ", t.settings_title),
        ];
        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
            .select(self.selected_tab)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs, header_chunks[1]);

        if self.selected_tab == 0 {
            self.render_dashboard(frame, chunks[1], &t);
        } else {
            self.render_settings(frame, chunks[1], &t);
        }

        self.render_footer(frame, chunks[2]);
    }

    fn render_dashboard(&self, frame: &mut Frame, area: Rect, t: &crate::i18n::Translation) {
        let state = self.monitor.get_state();
        let is_conn = state.is_connected.load(std::sync::atomic::Ordering::Relaxed);

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Status card
                Constraint::Length(7), // Gauges card
                Constraint::Min(5),    // Sparklines
            ])
            .split(area);

        // Hardware Status Line
        let conn_status = if is_conn {
            Span::styled(format!("🟢 {}", t.status_connected), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(format!("🟡 {}", t.status_searching), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        };

        let vid_pid_str = format!("0x{:04X}:0x{:04X}", self.custom_vid, self.custom_pid);
        let status_lines = vec![
            Line::from(vec![
                Span::raw("Device: "),
                Span::styled("ID-COOLING FX Series ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({})", vid_pid_str), Style::default().fg(Color::DarkGray)),
                Span::raw("  │  Status: "),
                conn_status,
                Span::raw("  │  Interval: "),
                Span::styled(format!("{} ms", self.update_interval_ms), Style::default().fg(Color::Cyan)),
            ]),
        ];

        let status_block = Paragraph::new(status_lines)
            .block(Block::default().title(format!(" {} ", t.device_name)).borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)));
        frame.render_widget(status_block, main_chunks[0]);

        // Live Telemetry Gauges
        let (temp_val, load_val) = if let Ok(m) = state.metrics.lock() {
            (
                m.temperature.map(|t| t.round() as i32).unwrap_or(0),
                m.load_percent.map(|l| l.round() as i32).unwrap_or(0),
            )
        } else {
            (0, 0)
        };

        let broadcast_val = if let Ok(v) = state.broadcast_value.lock() {
            v.clone()
        } else {
            "-".to_string()
        };

        let metrics_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(main_chunks[1]);

        let temp_gauge = Gauge::default()
            .block(Block::default().title(format!(" {} ", t.chip_temp)))
            .gauge_style(Style::default().fg(Color::Red).bg(Color::DarkGray))
            .percent(temp_val.clamp(0, 100) as u16)
            .label(format!("{} °C", temp_val));
        frame.render_widget(temp_gauge, metrics_chunks[0]);

        let load_gauge = Gauge::default()
            .block(Block::default().title(format!(" {} (Display Output: {}) ", t.chip_load, broadcast_val)))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
            .percent(load_val.clamp(0, 100) as u16)
            .label(format!("{} %", load_val));
        frame.render_widget(load_gauge, metrics_chunks[1]);

        // History Sparklines
        let spark_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[2]);

        let temp_spark = Sparkline::default()
            .block(Block::default().title(" Temperature Trend (°C) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Red)))
            .data(&self.temp_history)
            .max(100)
            .style(Style::default().fg(Color::Red));
        frame.render_widget(temp_spark, spark_chunks[0]);

        let load_spark = Sparkline::default()
            .block(Block::default().title(" CPU Load Trend (%) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Green)))
            .data(&self.load_history)
            .max(100)
            .style(Style::default().fg(Color::Green));
        frame.render_widget(load_spark, spark_chunks[1]);
    }

    fn render_settings(&self, frame: &mut Frame, area: Rect, t: &crate::i18n::Translation) {
        let mode_str = if self.display_mode == "temp" { &t.mode_temp } else { &t.mode_load };
        let src_str = match self.temp_source.as_str() {
            "core0" => &t.temp_src_core0,
            "avg" => &t.temp_src_avg,
            "max" => &t.temp_src_max,
            _ => &t.temp_src_package,
        };
        let smoothing_str = if self.temp_smoothing == 0 {
            t.smoothing_off.clone()
        } else {
            format!("{}°C", self.temp_smoothing)
        };
        let lang_str = match self.language.as_str() {
            "ru" => "Русский (ru)",
            "zh" => "简体中文 (zh)",
            "de" => "Deutsch (de)",
            "fr" => "Français (fr)",
            _ => "English (en)",
        };
        let autostart_str = match self.autostart_mode.as_str() {
            "daemon" => "Daemon (~2 MB RAM)",
            "gui" => "GUI (in Tray)",
            _ => "Disabled",
        };
        let tray_str = if self.show_tray { "Enabled" } else { "Disabled" };

        let items = vec![
            format!("{}: < {} >", t.setting_display_mode, mode_str),
            format!("{}: < {} ms >", t.setting_interval, self.update_interval_ms),
            format!("{}: < {} >", t.setting_temp_source, src_str),
            format!("{}: < {} >", t.setting_temp_smoothing, smoothing_str),
            format!("{}: < {} >", t.setting_language, lang_str),
            format!("Target USB VID: < 0x{:04X} >", self.custom_vid),
            format!("Target USB PID: < 0x{:04X} >", self.custom_pid),
            format!("{}: < {} >", t.setting_autostart, autostart_str),
            format!("System Tray Icon: < {} >", tray_str),
            format!("  [ {} (S) ]", t.btn_save_return),
            format!("  [ {} (R) ]", t.setting_reset_defaults),
            "  [ Power Off Display & Stop (X) ]".to_string(),
        ];

        let list_items: Vec<ListItem> = items
            .into_iter()
            .enumerate()
            .map(|(idx, item)| {
                let style = if idx == self.selected_setting {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(item).style(style)
            })
            .collect();

        let list = List::new(list_items)
            .block(Block::default().title(format!(" {} ", t.settings_title)).borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));

        frame.render_widget(list, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let msg_line = if let Some((ref msg, time)) = self.status_message {
            if time.elapsed() < Duration::from_secs(4) {
                Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
            } else {
                Line::from(Span::raw(""))
            }
        } else {
            Line::from(Span::raw(""))
        };

        let keys_text = " [Tab] View | [↑/↓] Select | [←/→] Change | [S] Save | [Q] Exit (Daemon in background) | [X] Stop";
        let footer_widget = Paragraph::new(vec![
            msg_line,
            Line::from(Span::styled(keys_text, Style::default().fg(Color::Cyan))),
        ])
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

        frame.render_widget(footer_widget, area);
    }
}
