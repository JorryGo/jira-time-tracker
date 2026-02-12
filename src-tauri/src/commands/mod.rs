pub mod jira;
pub mod settings;
pub mod timer;
pub mod worklogs;

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
