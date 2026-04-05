// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod osc;

use config::AppConfig;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::Manager;
use tokio::sync::watch;

struct OscState {
    /// Send a new port value to restart the listener
    port_tx: watch::Sender<u16>,
}

#[tauri::command]
fn restart_osc(
    osc_state: tauri::State<'_, OscState>,
    config: tauri::State<'_, AppConfig>,
) -> Result<(), String> {
    let port = config.settings.lock().map_err(|e| e.to_string())?.udp_port;
    osc_state.port_tx.send(port).map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let app_config = AppConfig::new();
    let initial_port = app_config.settings.lock().unwrap().udp_port;

    let (port_tx, port_rx) = watch::channel(initial_port);

    tauri::Builder::default()
        .manage(app_config)
        .manage(OscState { port_tx })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            restart_osc,
        ])
        .menu(|handle| {
            let app_menu = SubmenuBuilder::new(handle, "sndwrks hud")
                .about(None)
                .separator()
                .item(
                    &MenuItemBuilder::with_id("settings", "Settings...")
                        .accelerator("CmdOrCtrl+,")
                        .build(handle)?,
                )
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let edit_menu = SubmenuBuilder::new(handle, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let window_menu = SubmenuBuilder::new(handle, "Window")
                .minimize()
                .separator()
                .close_window()
                .build()?;

            MenuBuilder::new(handle)
                .item(&app_menu)
                .item(&edit_menu)
                .item(&window_menu)
                .build()
        })
        .setup(move |app| {
            // Hide settings and help windows on close instead of destroying them
            for label in &["settings", "help"] {
                if let Some(window) = app.get_webview_window(label) {
                    let win = window.clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = win.hide();
                        }
                    });
                }
            }

            // Restore saved HUD window position and track moves
            if let Some(hud_window) = app.get_webview_window("hud") {
                let settings = app.state::<AppConfig>().settings.lock().unwrap().clone();
                if let (Some(x), Some(y)) = (settings.hud_x, settings.hud_y) {
                    let _ = hud_window.set_position(tauri::PhysicalPosition::new(x, y));
                }

                let app_handle_for_move = app.handle().clone();
                hud_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Moved(position) = event {
                        let config = app_handle_for_move.state::<AppConfig>();
                        if let Ok(mut s) = config.settings.lock() {
                            s.hud_x = Some(position.x);
                            s.hud_y = Some(position.y);
                        }
                        let _ = config.save();
                    }
                });
            }

            // Handle menu events
            app.on_menu_event(move |app_handle, event| {
                if event.id().as_ref() == "settings" {
                    if let Some(window) = app_handle.get_webview_window("settings") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            });

            let app_handle = app.handle().clone();
            let mut port_rx = port_rx;

            tauri::async_runtime::spawn(async move {
                loop {
                    let port = *port_rx.borrow_and_update();
                    println!("Starting OSC UDP listener on port {}", port);

                    tokio::select! {
                        _ = osc::start_udp_listener(port, app_handle.clone()) => {
                            // Listener exited unexpectedly, will restart on next port change
                            eprintln!("OSC listener exited unexpectedly");
                        }
                        _ = port_rx.changed() => {
                            println!("Port changed, restarting OSC listener...");
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
