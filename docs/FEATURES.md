# What it can and can't do

## Can

- **System-wide dictation**: hold Option+Space in any app, speak, release — transcript
  pastes at the cursor. Hands-free toggle mode also available (Settings → push-to-talk off).
- **Fully offline**: after a one-time model download, nothing leaves the machine —
  audio, text, or telemetry. Verified with a network monitor.
- **Model choice**: 65-model catalog (Whisper family, Parakeet, Moonshine, Canary,
  SenseVoice…) plus custom models — drop any whisper.cpp-compatible `.gguf`/`.bin`
  into `~/Library/Application Support/com.pais.handy/models/`.
- **Voice setup (benchmark wizard)**: read three phrases once; every downloaded
  model transcribes the same recordings and is scored on accuracy (word error rate)
  and speed with YOUR voice — one click applies the recommended model. Models →
  Voice setup.
- **AI cleanup, locally** (off by default): filler removal, spoken punctuation,
  number formatting via Apple Intelligence (on-device) or Ollama (localhost). Own
  hotkey: Option+Shift+Space.
- **Custom words**: fuzzy phonetic correction of names/jargon (Kwame, Adjoa, cedis…),
  with one-click suggestions mined from your own dictation history.
- **Spoken punctuation**: "period", "comma", "new line" become symbols instantly —
  deterministic, no LLM, article-guarded ("the period" stays literal).
- **Command mode**: highlight text (or copy it), hold Ctrl+Shift+Space, speak an
  instruction ("make this formal") — the transformed text replaces it. Selection is
  read via the Accessibility API; clipboard is the fallback. Local LLM only; fails
  closed.
- **Live typing** (experimental, off by default): words appear in the target app while
  you speak; the final cleaned text re-settles on release. Streaming models only.
- **Per-app cleanup prompts**: different post-processing tone per target app.
- **Noise suppression** (experimental, off by default): RNNoise pass before
  transcription for steady background noise.
- **Accent color**: re-tint the UI from Advanced settings.
- **History**: browser with search, audio playback, re-transcribe, star, retention limits.
- **Translate to English** (Whisper models).
- **Headless CLI**: `Handy.app/Contents/MacOS/handy --transcribe-file f.wav --model turbo
--json` — batch file transcription and model benchmarking without the GUI.
- **Self-healing hotkey** after macOS updates (see hotkey doctor in INSTALL.md).
- Audio feedback sounds/themes, clamshell (lid-closed) mic selection, live overlay,
  Silero VAD silence filtering, i18n UI.

## Can't / known limits

- **Secure input fields** (password boxes, Terminal secure mode): macOS blocks both
  the hotkey and insertion there by design. Not fixable.
- **Ad-hoc signing**: rebuilding from source invalidates macOS permission grants —
  re-grant after every rebuild (release builds from the same dmg are unaffected).
- **Accent sensitivity varies by model**: Parakeet V3 performs poorly on West African
  English (drops most words); use Whisper Turbo or an AfriSpeech fine-tune.
- **Low mic gain silently breaks everything**: below ~50/100 input volume all models
  hallucinate. No in-app warning yet (planned).
- No streaming type-as-you-talk yet, no meeting transcription, no wake word
  (deliberate — privacy). Command mode works on _copied_ text (AX selected-text is the
  documented upgrade path). Ctrl+Shift+Space may need rebinding if you use macOS
  input-source switching shortcuts.
- Apple Silicon only (this fork is built/tested on arm64; Intel needs its own build
  with dynamic ONNX Runtime — see upstream BUILD.md).
