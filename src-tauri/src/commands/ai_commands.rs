use crate::ai::{
    get_provider,
    types::{CompletionRequest, ModelInfo, StreamEventPayload},
};
use crate::app_state::AppState;
use crate::auth::keychain::{self, KeychainError};
use tauri::{command, AppHandle, Emitter, State};
use tokio::sync::{mpsc, oneshot};

/// Turn a keychain failure into an actionable message.
///
/// `NotFound` is a configuration problem; anything else (typical: the user
/// dismissed the macOS access dialog for a key saved by an older build) is an
/// ACL problem that the Settings → Privacy → “Fix” action resolves.
fn keychain_error(provider: &str, err: KeychainError) -> String {
    match err {
        KeychainError::NotFound => {
            format!("No API key found for '{provider}'. Open Settings → AI Providers and save one.")
        }
        _ => format!(
            "Could not read the '{provider}' key from the keychain ({err}). \
             Open Settings → Privacy → “Fix” once, then try again."
        ),
    }
}

#[command]
pub async fn list_models(provider: String) -> Result<Vec<ModelInfo>, String> {
    let api_key =
        keychain::get_credential(&provider).map_err(|err| keychain_error(&provider, err))?;

    let client = get_provider(&provider, api_key).map_err(|e| e.to_string())?;
    client.list_models().await.map_err(|e| e.to_string())
}

#[command]
pub async fn start_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CompletionRequest,
) -> Result<(), String> {
    let provider_name = request.provider.clone();
    let stream_id = request.stream_id.clone();

    let api_key = keychain::get_credential(&provider_name)
        .map_err(|err| keychain_error(&provider_name, err))?;

    let client = get_provider(&provider_name, api_key).map_err(|e| e.to_string())?;

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    let (event_tx, mut event_rx) = mpsc::channel::<StreamEventPayload>(64);

    state.register_stream(stream_id.clone(), cancel_tx).await;

    let app_clone = app.clone();
    let stream_id_clone = stream_id.clone();
    let app_state_clone = state.inner().clone();

    // Task that forwards events from the provider channel to Tauri UI events
    tokio::spawn(async move {
        let event_channel = format!("stream:{}", stream_id_clone);
        while let Some(event) = event_rx.recv().await {
            let is_final = matches!(
                event,
                StreamEventPayload::Done { .. } | StreamEventPayload::Error { .. }
            );
            if let Err(e) = app_clone.emit(&event_channel, &event) {
                tracing::error!("Failed to emit stream event: {}", e);
            }
            if is_final {
                break;
            }
        }
        app_state_clone.remove_stream(&stream_id_clone).await;
    });

    // Spawn provider stream runner
    tokio::spawn(async move {
        if let Err(e) = client.stream(request, event_tx.clone(), cancel_rx).await {
            tracing::error!("Stream error in provider: {}", e);
            let _ = event_tx
                .send(StreamEventPayload::Error {
                    message: e.to_string(),
                })
                .await;
        }
    });

    Ok(())
}

#[command]
pub async fn cancel_stream(state: State<'_, AppState>, stream_id: String) -> Result<bool, String> {
    let was_cancelled = state.cancel_stream(&stream_id).await;
    Ok(was_cancelled)
}
