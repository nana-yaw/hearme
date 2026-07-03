use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, write_settings, ModelUnloadTimeout};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Serialize, Type)]
pub struct ModelLoadStatus {
    is_loaded: bool,
    current_model: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub fn set_model_unload_timeout(app: AppHandle, timeout: ModelUnloadTimeout) {
    let mut settings = get_settings(&app);
    settings.model_unload_timeout = timeout;
    write_settings(&app, settings);
}

#[tauri::command]
#[specta::specta]
pub fn get_model_load_status(
    // State must name the managed type exactly — lib.rs manages
    // Arc<TranscriptionManager>, so State<TranscriptionManager> panics at
    // runtime. Latent upstream bug: these commands had no frontend callers
    // until the load/unload toggle.
    transcription_manager: State<Arc<TranscriptionManager>>,
) -> Result<ModelLoadStatus, String> {
    Ok(ModelLoadStatus {
        is_loaded: transcription_manager.is_model_loaded(),
        current_model: transcription_manager.get_current_model(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn unload_model_manually(
    transcription_manager: State<Arc<TranscriptionManager>>,
) -> Result<(), String> {
    transcription_manager
        .unload_model()
        .map_err(|e| format!("Failed to unload model: {}", e))
}

/// Load the already-selected model without changing the selection — the
/// counterpart to `unload_model_manually`, so the UI can offer an
/// activate/deactivate toggle instead of forcing a re-selection from the
/// dropdown. Progress reaches the frontend via the `model-state-changed`
/// events that `load_model` emits.
#[tauri::command]
#[specta::specta]
pub fn load_model_manually(
    app: AppHandle,
    transcription_manager: State<Arc<TranscriptionManager>>,
) -> Result<(), String> {
    let _loading_guard = transcription_manager
        .try_start_loading()
        .ok_or_else(|| "Model load already in progress".to_string())?;

    let model_id = get_settings(&app).selected_model;
    if model_id.is_empty() {
        return Err("No model selected".to_string());
    }

    transcription_manager
        .load_model(&model_id)
        .map_err(|e| format!("Failed to load model {}: {}", model_id, e))
}
