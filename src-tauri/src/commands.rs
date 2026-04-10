use crate::config::{AppConfig, Settings};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(config: State<'_, AppConfig>) -> Settings {
    config.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    config: State<'_, AppConfig>,
    settings: Settings,
) -> Result<(), String> {
    let always_on_top = {
        let mut current = config.settings.lock().map_err(|e| e.to_string())?;
        let saved_x = current.hud_x;
        let saved_y = current.hud_y;
        let saved_width = current.hud_width;
        let saved_height = current.hud_height;
        *current = settings;
        current.hud_x = saved_x;
        current.hud_y = saved_y;
        current.hud_width = saved_width;
        current.hud_height = saved_height;
        current.always_on_top
    };
    config.save()?;

    if let Some(hud_window) = app.get_webview_window("hud") {
        hud_window
            .set_always_on_top(always_on_top)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
