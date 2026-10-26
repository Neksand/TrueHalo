use crate::app_state::AppState;
use crate::storage::db::{MessageRecord, SessionRecord};
use tauri::{command, AppHandle, Manager, State};

#[command]
pub fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionRecord>, String> {
    state.db.list_sessions().map_err(|e| e.to_string())
}

#[command]
pub fn rename_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err("A conversation needs a title".to_string());
    }
    if trimmed.chars().count() > 120 {
        return Err("Keep the title under 120 characters".to_string());
    }
    state
        .db
        .rename_session(&session_id, trimmed)
        .map_err(|e| e.to_string())
}

#[command]
pub fn search_sessions(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SessionRecord>, String> {
    state
        .db
        .search_sessions(&query)
        .map_err(|e| e.to_string())
}

#[command]
pub fn create_session(
    state: State<'_, AppState>,
    id: String,
    title: String,
    provider: String,
    model: String,
) -> Result<SessionRecord, String> {
    state
        .db
        .create_session(&id, &title, &provider, &model)
        .map_err(|e| e.to_string())
}

#[command]
pub fn get_session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<MessageRecord>, String> {
    state
        .db
        .get_session_messages(&session_id)
        .map_err(|e| e.to_string())
}

#[command]
pub fn save_message(
    state: State<'_, AppState>,
    id: String,
    session_id: String,
    role: String,
    content_json: String,
    usage_json: Option<String>,
) -> Result<(), String> {
    state
        .db
        .save_message(&id, &session_id, &role, &content_json, usage_json.as_deref())
        .map_err(|e| e.to_string())
}

#[command]
pub fn usage_summary(state: State<'_, AppState>) -> Result<crate::storage::db::UsageSummary, String> {
    state.db.usage_summary().map_err(|e| e.to_string())
}

#[command]
pub fn delete_session(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    state
        .db
        .delete_session(&session_id)
        .map_err(|e| e.to_string())
}

#[command]
pub fn clear_all_history(state: State<'_, AppState>) -> Result<(), String> {
    state.db.clear_all_history().map_err(|e| e.to_string())
}

/// Write an exported conversation to the user's Downloads folder.
///
/// The UI builds the Markdown/JSON so it exports exactly what is on screen;
/// this command only picks a safe, unique filename and writes the bytes.
#[command]
pub fn save_export(app: AppHandle, filename: String, content: String) -> Result<String, String> {
    let lower = filename.to_ascii_lowercase();
    let extension = if lower.ends_with(".json") {
        "json"
    } else if lower.ends_with(".md") {
        "md"
    } else {
        return Err("Export files must end in .md or .json".to_string());
    };

    let stem = if extension == "json" {
        &filename[..filename.len() - ".json".len()]
    } else {
        &filename[..filename.len() - ".md".len()]
    };
    let mut safe_stem: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    safe_stem = safe_stem.trim_matches(['.', ' ']).trim().to_string();
    if safe_stem.is_empty() {
        safe_stem = "truehalo-export".to_string();
    }

    let directory = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| format!("Could not find a Downloads folder: {e}"))?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;

    let mut path = directory.join(format!("{safe_stem}.{extension}"));
    let mut counter = 2;
    while path.exists() {
        if counter > 999 {
            return Err("Too many exports with this name".to_string());
        }
        path = directory.join(format!("{safe_stem}-{counter}.{extension}"));
        counter += 1;
    }

    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[command]
pub fn set_preference(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    state
        .db
        .set_preference(&key, &value)
        .map_err(|e| e.to_string())
}

#[command]
pub fn get_preference(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state.db.get_preference(&key).map_err(|e| e.to_string())
}
