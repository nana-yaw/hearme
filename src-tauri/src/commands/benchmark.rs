use crate::audio_toolkit::{speech_level_dbfs, VadPolicy};
use crate::benchmark::{recommend, word_error_rate, BenchmarkModelResult};
use crate::managers::audio::AudioRecordingManager;
use crate::managers::model::ModelManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, ModelUnloadTimeout};
use log::{info, warn};
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};

/// Recording-manager binding id for wizard captures, so a benchmark recording
/// and a hotkey dictation can never stop each other's session.
const BENCHMARK_BINDING_ID: &str = "voice-setup-benchmark";

/// Same threshold as the dictation quiet-input warning in `actions.rs`.
const QUIET_INPUT_THRESHOLD_DBFS: f32 = -40.0;

/// How long a benchmark run waits for another model load (launch preload, a
/// dictation-triggered load) to finish before giving up on that model.
const LOADING_SLOT_TIMEOUT: Duration = Duration::from_secs(20);

/// Phrase recordings captured by the wizard, held in memory only — benchmark
/// AUDIO is never written to history or disk. Transcripts, however, flow
/// through the standard `transcribe()` pipeline, which logs them to the local
/// app log exactly like every dictation.
#[derive(Default)]
pub struct BenchmarkState {
    samples_by_phrase: Mutex<HashMap<u32, Vec<f32>>>,
    running: AtomicBool,
    cancel: AtomicBool,
}

impl BenchmarkState {
    /// True while the benchmark thread runs. The dictation hotkey path checks
    /// this: both sides share the single transcription engine slot, so a
    /// dictation during a benchmark would be transcribed by whichever
    /// benchmark model happens to be loaded.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
}

/// Clears the running flag even if the benchmark thread errors or panics.
struct RunningGuard(Arc<BenchmarkState>);

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::Release);
    }
}

#[derive(Serialize, Clone, Debug, Type)]
pub struct BenchmarkRecordingResult {
    pub duration_secs: f64,
    pub level_dbfs: Option<f32>,
    pub too_quiet: bool,
}

#[derive(Serialize, Clone, Debug, Type)]
pub struct BenchmarkProgressEvent {
    pub model_id: String,
    pub model_index: u32,
    pub total_models: u32,
    pub phrase_index: u32,
    pub total_phrases: u32,
    /// "loading" while the model loads, "transcribing" per phrase.
    pub stage: String,
}

#[derive(Serialize, Clone, Debug, Type)]
pub struct BenchmarkCompleteEvent {
    pub results: Vec<BenchmarkModelResult>,
    pub recommended_model_id: Option<String>,
    pub cancelled: bool,
}

// The recording commands are async so Tauri runs them off the main thread:
// stop_recording can sleep for `extra_recording_buffer_ms` and starting can
// open the microphone stream — the dictation path keeps the same calls off
// the main thread for the same reason.
#[tauri::command]
#[specta::specta]
pub async fn benchmark_start_recording(
    recording_manager: State<'_, Arc<AudioRecordingManager>>,
) -> Result<(), String> {
    recording_manager.try_start_recording(BENCHMARK_BINDING_ID, VadPolicy::Offline)
}

#[tauri::command]
#[specta::specta]
pub async fn benchmark_stop_recording(
    recording_manager: State<'_, Arc<AudioRecordingManager>>,
    state: State<'_, Arc<BenchmarkState>>,
    phrase_index: u32,
) -> Result<BenchmarkRecordingResult, String> {
    let samples = recording_manager
        .stop_recording(BENCHMARK_BINDING_ID, recording_manager.cancel_generation())
        .ok_or_else(|| "No active benchmark recording".to_string())?;

    let duration_secs = samples.len() as f64 / 16_000.0;
    let level_dbfs = speech_level_dbfs(&samples, 16_000);
    let too_quiet = level_dbfs.is_some_and(|level| level < QUIET_INPUT_THRESHOLD_DBFS);

    state
        .samples_by_phrase
        .lock()
        .unwrap()
        .insert(phrase_index, samples);

    Ok(BenchmarkRecordingResult {
        duration_secs,
        level_dbfs,
        too_quiet,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn benchmark_cancel_recording(
    recording_manager: State<'_, Arc<AudioRecordingManager>>,
) -> Result<(), String> {
    // Binding-scoped discard: the global cancel_recording() bumps a
    // process-wide cancel generation, which would silently abort a dictation
    // pipeline still finishing in the background. stop_recording checks the
    // binding id, so this ends only the wizard's own session; the returned
    // samples are dropped unstored.
    let _ = recording_manager
        .stop_recording(BENCHMARK_BINDING_ID, recording_manager.cancel_generation());
    Ok(())
}

/// Drop all captured phrase audio (wizard closed or restarted).
#[tauri::command]
#[specta::specta]
pub fn benchmark_discard_samples(state: State<Arc<BenchmarkState>>) {
    state.samples_by_phrase.lock().unwrap().clear();
}

#[tauri::command]
#[specta::specta]
pub fn cancel_model_benchmark(state: State<Arc<BenchmarkState>>) {
    state.cancel.store(true, Ordering::Release);
}

/// Run every requested model over the captured phrase recordings, emitting
/// `benchmark-progress` per step and `benchmark-complete` with scored results.
/// Returns immediately; the run continues on a background thread. Models load
/// strictly one at a time (bounded memory) and the user's selected model is
/// restored afterwards.
#[tauri::command]
#[specta::specta]
pub fn run_model_benchmark(
    app: AppHandle,
    state: State<Arc<BenchmarkState>>,
    recording_manager: State<Arc<AudioRecordingManager>>,
    model_ids: Vec<String>,
    references: Vec<String>,
) -> Result<(), String> {
    if model_ids.is_empty() {
        return Err("No models selected for the benchmark".to_string());
    }
    if references.is_empty() {
        return Err("No reference phrases provided".to_string());
    }
    // Mirror of the hotkey-side guard: a dictation in flight and a benchmark
    // would race for the single transcription engine slot.
    if recording_manager.is_recording() {
        return Err("A dictation is in progress — finish it first".to_string());
    }
    if state.running.swap(true, Ordering::AcqRel) {
        return Err("A benchmark is already running".to_string());
    }
    let guard = RunningGuard(Arc::clone(&state));
    state.cancel.store(false, Ordering::Release);

    // Snapshot the recordings up front so a missing phrase fails the command,
    // not the background thread.
    let samples: Vec<Vec<f32>> = {
        let stored = state.samples_by_phrase.lock().unwrap();
        (0..references.len() as u32)
            .map(|index| {
                stored
                    .get(&index)
                    .cloned()
                    .ok_or_else(|| format!("Missing recording for phrase {}", index + 1))
            })
            .collect::<Result<_, _>>()?
    };

    let state_for_thread = Arc::clone(&state);
    std::thread::spawn(move || {
        let _running = guard;
        run_benchmark_thread(app, state_for_thread, model_ids, references, samples);
    });
    Ok(())
}

fn run_benchmark_thread(
    app: AppHandle,
    state: Arc<BenchmarkState>,
    model_ids: Vec<String>,
    references: Vec<String>,
    samples: Vec<Vec<f32>>,
) {
    let transcription_manager = app.state::<Arc<TranscriptionManager>>().inner().clone();
    let model_manager = app.state::<Arc<ModelManager>>().inner().clone();
    let user_selected_model = get_settings(&app).selected_model;

    let total_models = model_ids.len() as u32;
    let total_phrases = references.len() as u32;
    let mut results: Vec<BenchmarkModelResult> = Vec::new();
    let cancelled = |state: &BenchmarkState| state.cancel.load(Ordering::Acquire);

    'models: for (model_index, model_id) in model_ids.iter().enumerate() {
        if cancelled(&state) {
            break;
        }
        let model_name = model_manager
            .get_model_info(model_id)
            .map(|info| info.name)
            .unwrap_or_else(|| model_id.clone());
        let emit_progress = |phrase_index: u32, stage: &str| {
            let _ = app.emit(
                "benchmark-progress",
                BenchmarkProgressEvent {
                    model_id: model_id.clone(),
                    model_index: model_index as u32,
                    total_models,
                    phrase_index,
                    total_phrases,
                    stage: stage.to_string(),
                },
            );
        };
        let errored = |message: String| BenchmarkModelResult {
            model_id: model_id.clone(),
            model_name: model_name.clone(),
            accuracy: 0.0,
            speed_factor: 0.0,
            load_ms: 0,
            transcripts: Vec::new(),
            error: Some(message),
        };

        emit_progress(0, "loading");
        let load_started = Instant::now();
        match load_model_serialized(&transcription_manager, &state, model_id) {
            Ok(()) => {}
            Err(message) => {
                warn!(
                    "Benchmark: model '{}' failed to load: {}",
                    model_id, message
                );
                results.push(errored(message));
                continue;
            }
        }
        let load_ms = load_started.elapsed().as_millis() as u64;

        let mut transcripts: Vec<String> = Vec::new();
        let mut accuracy_sum = 0.0;
        let mut speed_sum = 0.0;
        for (phrase_index, (audio, reference)) in samples.iter().zip(&references).enumerate() {
            if cancelled(&state) {
                break 'models;
            }
            emit_progress(phrase_index as u32, "transcribing");

            // The "Immediately" unload policy drops the engine after every
            // transcription — reload between phrases so the policy stays
            // honored without failing the benchmark.
            if !transcription_manager.is_model_loaded() {
                if let Err(message) =
                    load_model_serialized(&transcription_manager, &state, model_id)
                {
                    results.push(errored(message));
                    continue 'models;
                }
            }

            let transcribe_started = Instant::now();
            match transcription_manager.transcribe(audio.clone()) {
                Ok(transcript) => {
                    let compute_secs = transcribe_started.elapsed().as_secs_f64();
                    let audio_secs = audio.len() as f64 / 16_000.0;
                    accuracy_sum += 1.0 - word_error_rate(reference, &transcript);
                    speed_sum += if compute_secs > 0.0 {
                        audio_secs / compute_secs
                    } else {
                        0.0
                    };
                    transcripts.push(transcript);
                }
                Err(e) => {
                    let message = format!("Transcription failed: {}", e);
                    warn!("Benchmark: model '{}': {}", model_id, message);
                    results.push(errored(message));
                    continue 'models;
                }
            }
        }

        let phrase_count = transcripts.len().max(1) as f64;
        info!(
            "Benchmark: model '{}' scored accuracy {:.3}, speed {:.2}x, load {}ms",
            model_id,
            accuracy_sum / phrase_count,
            speed_sum / phrase_count,
            load_ms
        );
        results.push(BenchmarkModelResult {
            model_id: model_id.clone(),
            model_name,
            accuracy: accuracy_sum / phrase_count,
            speed_factor: speed_sum / phrase_count,
            load_ms,
            transcripts,
            error: None,
        });
    }

    restore_user_model(&app, &transcription_manager, &user_selected_model);

    let was_cancelled = cancelled(&state);
    let recommended_model_id = if was_cancelled {
        None
    } else {
        recommend(&results)
    };
    let _ = app.emit(
        "benchmark-complete",
        BenchmarkCompleteEvent {
            results,
            recommended_model_id,
            cancelled: was_cancelled,
        },
    );
}

/// Load a model under the manager's loading flag, waiting out any in-flight
/// load (launch preload, dictation auto-load) up to [`LOADING_SLOT_TIMEOUT`].
fn load_model_serialized(
    transcription_manager: &TranscriptionManager,
    state: &BenchmarkState,
    model_id: &str,
) -> Result<(), String> {
    let waited = Instant::now();
    let _loading_guard = loop {
        if let Some(loading_guard) = transcription_manager.try_start_loading() {
            break loading_guard;
        }
        if state.cancel.load(Ordering::Acquire) {
            return Err("Benchmark cancelled".to_string());
        }
        if waited.elapsed() > LOADING_SLOT_TIMEOUT {
            return Err("Timed out waiting for another model load to finish".to_string());
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    transcription_manager
        .load_model(model_id)
        .map_err(|e| e.to_string())
}

/// Put the app back the way the user had it: unload under the "Immediately"
/// policy, otherwise reload the selected model if the benchmark left a
/// different one in memory.
fn restore_user_model(
    app: &AppHandle,
    transcription_manager: &TranscriptionManager,
    user_selected_model: &str,
) {
    if get_settings(app).model_unload_timeout == ModelUnloadTimeout::Immediately {
        let _ = transcription_manager.unload_model();
        return;
    }
    if user_selected_model.is_empty()
        || transcription_manager.get_current_model().as_deref() == Some(user_selected_model)
    {
        return;
    }
    if let Some(_loading_guard) = transcription_manager.try_start_loading() {
        if let Err(e) = transcription_manager.load_model(user_selected_model) {
            warn!(
                "Benchmark: failed to restore selected model '{}': {}",
                user_selected_model, e
            );
        }
    }
}
