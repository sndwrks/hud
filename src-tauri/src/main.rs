// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod osc;

macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        println!($($arg)*);
    };
}

pub(crate) use debug_log;

use config::AppConfig;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::Manager;
use tokio::sync::watch;

struct OscState {
    /// Send a new port value to restart the UDP listener
    udp_port_tx: watch::Sender<u16>,
    /// Send a new port value to restart the TCP listener
    tcp_port_tx: watch::Sender<u16>,
}

#[tauri::command]
fn restart_osc(
    osc_state: tauri::State<'_, OscState>,
    config: tauri::State<'_, AppConfig>,
) -> Result<(), String> {
    let (udp_port, tcp_port) = {
        let s = config.settings.lock().map_err(|e| e.to_string())?;
        (s.udp_port, s.tcp_port)
    };
    osc_state
        .udp_port_tx
        .send(udp_port)
        .map_err(|e| e.to_string())?;
    osc_state
        .tcp_port_tx
        .send(tcp_port)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let app_config = AppConfig::new();
    let (initial_udp_port, initial_tcp_port) = {
        let s = app_config.settings.lock().unwrap();
        (s.udp_port, s.tcp_port)
    };

    let (udp_port_tx, udp_port_rx) = watch::channel(initial_udp_port);
    let (tcp_port_tx, tcp_port_rx) = watch::channel(initial_tcp_port);

    tauri::Builder::default()
        .manage(app_config)
        .manage(OscState {
            udp_port_tx,
            tcp_port_tx,
        })
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
                let _ = hud_window.set_size(tauri::PhysicalSize::new(settings.hud_width, settings.hud_height));
                let _ = hud_window.set_always_on_top(settings.always_on_top);

                let app_handle_for_move = app.handle().clone();
                hud_window.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::Moved(position) => {
                            let config = app_handle_for_move.state::<AppConfig>();
                            if let Ok(mut s) = config.settings.lock() {
                                s.hud_x = Some(position.x);
                                s.hud_y = Some(position.y);
                            }
                            let _ = config.save();
                        }
                        tauri::WindowEvent::Resized(size) => {
                            let config = app_handle_for_move.state::<AppConfig>();
                            if let Ok(mut s) = config.settings.lock() {
                                s.hud_width = size.width;
                                s.hud_height = size.height;
                            }
                            let _ = config.save();
                        }
                        _ => {}
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

            let app_handle_udp = app.handle().clone();
            let mut udp_port_rx = udp_port_rx;

            tauri::async_runtime::spawn(async move {
                loop {
                    let port = *udp_port_rx.borrow_and_update();
                    debug_log!("Starting OSC UDP listener on port {}", port);

                    tokio::select! {
                        _ = osc::start_udp_listener(port, app_handle_udp.clone()) => {
                            eprintln!("OSC UDP listener exited unexpectedly");
                        }
                        _ = udp_port_rx.changed() => {
                            debug_log!("UDP port changed, restarting OSC UDP listener...");
                        }
                    }
                }
            });

            let app_handle_tcp = app.handle().clone();
            let mut tcp_port_rx = tcp_port_rx;

            tauri::async_runtime::spawn(async move {
                loop {
                    let port = *tcp_port_rx.borrow_and_update();
                    debug_log!("Starting OSC TCP listener on port {}", port);

                    tokio::select! {
                        _ = osc::start_tcp_listener(port, app_handle_tcp.clone()) => {
                            eprintln!("OSC TCP listener exited unexpectedly");
                        }
                        _ = tcp_port_rx.changed() => {
                            debug_log!("TCP port changed, restarting OSC TCP listener...");
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
