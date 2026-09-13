use crate::i18n::I18n;
use muda::{Menu, MenuEvent, MenuItem};
use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct SystemTray {
    _tray_icon: TrayIcon,
    show_item_id: muda::MenuId,
    exit_item_id: muda::MenuId,
    show_item: MenuItem,
    exit_item: MenuItem,
    is_visible: AtomicBool,
}

impl SystemTray {
    pub fn new(initial_visible: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let t = I18n::get();
        let tray_menu = Menu::new();
        let show_label = if initial_visible {
            &t.tray_hide
        } else {
            &t.tray_show
        };
        let show_item = MenuItem::new(show_label, true, None);
        let show_item_id = show_item.id().clone();
        let exit_item = MenuItem::new(&t.tray_exit, true, None);
        let exit_item_id = exit_item.id().clone();

        // 2 compact items: Show/Hide and Exit (no separator)
        tray_menu.append(&show_item)?;
        tray_menu.append(&exit_item)?;

        // Apply native dark theme
        apply_menu_dark_theme();

        let icon = create_default_icon()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("RustCooling")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            show_item_id,
            exit_item_id,
            show_item,
            exit_item,
            is_visible: AtomicBool::new(initial_visible),
        })
    }

    pub fn set_window_visible(&self, visible: bool) {
        self.is_visible.store(visible, Ordering::SeqCst);
        let t = I18n::get();
        let label = if visible { &t.tray_hide } else { &t.tray_show };
        self.show_item.set_text(label);
    }

    pub fn update_labels(&self) {
        let t = I18n::get();
        let visible = self.is_visible.load(Ordering::SeqCst);
        let label = if visible { &t.tray_hide } else { &t.tray_show };
        self.show_item.set_text(label);
        self.exit_item.set_text(&t.tray_exit);
    }

    pub fn poll_events<FToggle, FExit>(&self, on_toggle: FToggle, on_exit: FExit)
    where
        FToggle: Fn(),
        FExit: Fn(),
    {
        // Poll tray icon clicks (left click toggles the window)
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_toggle();
            }
        }

        // Poll menu items (right click context menu)
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_item_id {
                on_toggle();
            } else if event.id == self.exit_item_id {
                on_exit();
            }
        }
    }
}

#[cfg(windows)]
fn apply_menu_dark_theme() {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};

    unsafe {
        // Force Windows Dark Mode for context menus (native dark acrylic / black theme)
        let uxtheme = LoadLibraryA(c"uxtheme.dll".as_ptr() as *const u8);
        if !uxtheme.is_null() {
            let set_preferred_app_mode: Option<unsafe extern "system" fn(i32) -> i32> =
                std::mem::transmute(GetProcAddress(uxtheme, 135 as _));
            if let Some(func) = set_preferred_app_mode {
                func(2); // 2 = ForceDark
            } else {
                let allow_dark_mode: Option<unsafe extern "system" fn(bool) -> bool> =
                    std::mem::transmute(GetProcAddress(uxtheme, 132 as _));
                if let Some(func) = allow_dark_mode {
                    func(true);
                }
            }
            let flush_menu_themes: Option<unsafe extern "system" fn()> =
                std::mem::transmute(GetProcAddress(uxtheme, 136 as _));
            if let Some(func) = flush_menu_themes {
                func();
            }
        }
    }
}

#[cfg(not(windows))]
fn apply_menu_dark_theme() {}

/// Loads the native 32x32 logo icon from embedded PE resources on Windows or RGBA bytes on Linux
fn create_default_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        if let Ok(icon) = Icon::from_resource(1, Some((32, 32))) {
            return Ok(icon);
        }
    }

    let rgba_bytes = include_bytes!("../../assets/icons/tray_32x32.rgba");
    let icon = Icon::from_rgba(rgba_bytes.to_vec(), 32, 32)?;
    Ok(icon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_icon() {
        let icon_res = create_default_icon();
        assert!(icon_res.is_ok());
    }

    #[test]
    fn test_tray_menu_dark_theme() {
        apply_menu_dark_theme();
    }
}
