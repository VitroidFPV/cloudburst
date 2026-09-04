use tauri::AppHandle;

const DEVELOPMENT_IDENTIFIER: &str = "dev.vitroid.cloudburst.dev";

pub fn is_development(app: &AppHandle) -> bool {
    app.config().identifier == DEVELOPMENT_IDENTIFIER
}
