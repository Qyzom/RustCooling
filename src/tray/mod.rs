use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use crate::i18n::I18n;

pub struct SystemTray {
    _tray_icon: TrayIcon,
    show_item_id: muda::MenuId,
    exit_item_id: muda::MenuId,
    show_item: MenuItem,
    exit_item: MenuItem,
}

impl SystemTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let t = I18n::get();
        let tray_menu = Menu::new();
        let show_item = MenuItem::new(&t.tray_show, true, None);
        let show_item_id = show_item.id().clone();
        let exit_item = MenuItem::new(&t.tray_exit, true, None);
        let exit_item_id = exit_item.id().clone();

        tray_menu.append(&show_item)?;
        tray_menu.append(&PredefinedMenuItem::separator())?;
        tray_menu.append(&exit_item)?;

        let icon = create_default_icon()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("RustCooling - ID-COOLING FX Controller")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            show_item_id,
            exit_item_id,
            show_item,
            exit_item,
        })
    }

    pub fn update_labels(&self) {
        let t = I18n::get();
        self.show_item.set_text(&t.tray_show);
        self.exit_item.set_text(&t.tray_exit);
    }

    pub fn poll_events<FShow, FExit>(&self, on_show: FShow, on_exit: FExit)
    where
        FShow: Fn(),
        FExit: Fn(),
    {
        // Poll tray icon clicks (left click restores/shows the window)
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_show();
            }
        }

        // Poll menu items (right click context menu)
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_item_id {
                on_show();
            } else if event.id == self.exit_item_id {
                on_exit();
            }
        }
    }
}

/// Generates a crisp 32x32 RGBA cooling icon (cyan/blue circular badge)
fn create_default_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let dx = (x as i32 - 16).abs();
            let dy = (y as i32 - 16).abs();
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= 14 * 14 {
                if dist_sq <= 11 * 11 {
                    // Inner mint teal
                    rgba.extend_from_slice(&[123, 208, 193, 255]); // #7bd0c1
                } else {
                    // Border dark slate
                    rgba.extend_from_slice(&[39, 42, 56, 255]); // #272a38
                }
            } else {
                // Transparent
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    let icon = Icon::from_rgba(rgba, width, height)?;
    Ok(icon)
}
