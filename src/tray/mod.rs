use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub struct SystemTray {
    _tray_icon: TrayIcon,
    show_item_id: muda::MenuId,
    exit_item_id: muda::MenuId,
}

impl SystemTray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Show / Hide Window", true, None);
        let show_item_id = show_item.id().clone();
        let exit_item = MenuItem::new("Exit", true, None);
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
        })
    }

    pub fn poll_events<FShow, FExit>(&self, on_toggle_show: FShow, on_exit: FExit)
    where
        FShow: Fn(),
        FExit: Fn(),
    {
        // Poll tray icon clicks (left click to show/hide)
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_toggle_show();
            }
        }

        // Poll menu items
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_item_id {
                on_toggle_show();
            } else if event.id == self.exit_item_id {
                on_exit();
            }
        }
    }
}

/// Generates a simple 32x32 RGBA cooling icon (cyan/blue square with accent) in pure code
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
                    // Inner cyan
                    rgba.extend_from_slice(&[137, 220, 235, 255]); // #89dceb
                } else {
                    // Border blue
                    rgba.extend_from_slice(&[137, 180, 250, 255]); // #89b4fa
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




