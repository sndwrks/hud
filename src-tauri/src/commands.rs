use crate::config::{AppConfig, Settings};
use tauri::State;

#[tauri::command]
pub fn get_settings(config: State<'_, AppConfig>) -> Settings {
    config.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(config: State<'_, AppConfig>, settings: Settings) -> Result<(), String> {
    {
        let mut current = config.settings.lock().map_err(|e| e.to_string())?;
        let saved_x = current.hud_x;
        let saved_y = current.hud_y;
        *current = settings;
        current.hud_x = saved_x;
        current.hud_y = saved_y;
    }
    config.save()
}
