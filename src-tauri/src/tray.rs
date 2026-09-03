use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Window, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const SHOW_MENU_ID: &str = "tray.show";
const QUIT_MENU_ID: &str = "tray.quit";

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let app_name = app
        .config()
        .product_name
        .as_deref()
        .unwrap_or("Cloudburst")
        .to_string();
    let show = MenuItem::with_id(
        app,
        SHOW_MENU_ID,
        format!("Show {app_name}"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        QUIT_MENU_ID,
        format!("Quit {app_name}"),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut tray = TrayIconBuilder::new()
        .tooltip(app_name)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            SHOW_MENU_ID => restore_window_main(app),
            QUIT_MENU_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                restore_window_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();

        if let Err(error) = window.hide() {
            eprintln!("failed to hide Cloudburst in the system tray: {error}");
        }
    }
}

/// Shows and focuses the main window; used by the tray and the
/// single-instance handler when a second instance is launched.
pub fn restore_window_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        eprintln!("failed to show Cloudburst: the main window does not exist");
        return;
    };

    // Some Linux window managers do not support unminimizing, but a failed
    // attempt should not prevent a hidden window from being shown.
    let _ = window.unminimize();
    if let Err(error) = window.show() {
        eprintln!("failed to show Cloudburst from the system tray: {error}");
        return;
    }
    if let Err(error) = window.set_focus() {
        eprintln!("failed to focus Cloudburst: {error}");
    }
}
