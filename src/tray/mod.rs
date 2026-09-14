use crate::i18n::I18n;
use muda::{Menu, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    Toggle,
    Exit,
}

pub struct SystemTray {
    _tray_icon: TrayIcon,
    show_item: MenuItem,
    exit_item: MenuItem,
}

impl SystemTray {
    pub fn new<F>(on_action: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: Fn(TrayAction) + Send + Sync + 'static,
    {
        let t = I18n::get();
        let tray_menu = Menu::new();
        let show_item = MenuItem::new(&t.tray_show, true, None);
        let show_item_id = show_item.id().clone();
        let exit_item = MenuItem::new(&t.tray_exit, true, None);
        let exit_item_id = exit_item.id().clone();

        // 2 compact items: Show and Exit
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

        let action_cb = std::sync::Arc::new(on_action);

        let cb_menu = std::sync::Arc::clone(&action_cb);
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if event.id == show_item_id {
                cb_menu(TrayAction::Show);
            } else if event.id == exit_item_id {
                cb_menu(TrayAction::Exit);
            }
        }));

        let cb_tray = std::sync::Arc::clone(&action_cb);
        tray_icon::TrayIconEvent::set_event_handler(Some(move |event: tray_icon::TrayIconEvent| {
            match event {
                tray_icon::TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    cb_tray(TrayAction::Toggle);
                }
                tray_icon::TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    cb_tray(TrayAction::Show);
                }
                _ => {}
            }
        }));

        Ok(Self {
            _tray_icon: tray_icon,
            show_item,
            exit_item,
        })
    }

    pub fn update_labels(&self) {
        let t = I18n::get();
        self.show_item.set_text(&t.tray_show);
        self.exit_item.set_text(&t.tray_exit);
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

    #[test]
    fn test_event_handlers_exist() {
        muda::MenuEvent::set_event_handler(Some(|_event: muda::MenuEvent| {}));
        tray_icon::TrayIconEvent::set_event_handler(Some(|event: tray_icon::TrayIconEvent| {
            match event {
                tray_icon::TrayIconEvent::Click { .. } => {}
                tray_icon::TrayIconEvent::DoubleClick { .. } => {}
                _ => {}
            }
        }));
    }
}
