use crate::app_state::AppState;
use crate::storage::db::{MessageRecord, SessionRecord};
use tauri::{command, State};

#[command]
pub fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionRecord>, String> {
    state.db.list_sessions().map_err(|e| e.to_string())
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
) -> Result<(), String> {
    state
        .db
        .save_message(&id, &session_id, &role, &content_json)
        .map_err(|e| e.to_string())
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
