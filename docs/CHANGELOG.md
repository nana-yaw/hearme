# Fork changelog

Versioning: `<upstream-version>-local.<n>` — the upstream Handy release the fork is
based on, plus a fork increment.

## v0.12.1 — 2026-07-03

### Fixed

- **Accent color now re-tints ALL interactive states.** The theme has two brand
  tokens: `--color-logo-primary` (chips, rings, highlights) and
  `--color-background-ui` (checked toggles, primary buttons). The accent picker
  only overrode the first, so active toggles and buttons stayed brand-pink under
  any custom accent. The strong color is now derived from the chosen accent using
  the brand pair's own relationship (S×0.72, L clamped to 0.60 — which maps the
  default #faa2ca to #db5793, one hex digit from the hand-picked #da5893), so
  every page re-tints consistently and white text keeps its contrast.

## v0.12.0 — 2026-07-02

### Added

- **Selected-text command mode**: command mode (Ctrl+Shift+Space) now reads the
  highlighted text in the frontmost app via the Accessibility API — no copy step.
  Selection wins over the clipboard; the clipboard remains the fallback, so the
  old workflow still works everywhere (including apps that don't expose
  `AXSelectedText`). The paste replaces the live selection in place.
- **Live typing** (experimental, default off, Advanced): during a streaming
  dictation, committed words type into the target app while you speak. On
  release the typed prefix is erased and the standard paste delivers the final
  text, so custom words, spoken punctuation, and AI edits always win. Typing
  halts instantly if you focus another app, and a cancelled dictation erases
  what it typed. Streaming-capable models only.

### Review hardening (14 confirmed findings, fixed before tag)

- Command mode never live-types — the spoken instruction would have replaced
  the highlighted selection before the AX read (high).
- Selection is captured at shortcut RELEASE, not after transcription, so focus
  changes during the pipeline can't swap in the wrong text; AX messaging
  timeout set to 0.5 s so a hung app can't stall the shortcut thread.
- Live typing focus guard upgraded from bundle-id to AX focused-element
  identity — window/tab/field switches inside the same app now halt typing;
  no capturable element (or non-macOS) keeps live typing off entirely.
- Erase is provably exact: only printable-ASCII deltas are live-typed (one
  scalar = one macOS deletion unit — verified empirically); anything else
  freezes live output and the final paste delivers it.
- Erase runs in chunks with focus re-checks and a deadline; a wedged typing
  queue can never fire deferred backspaces into the pasted text.
- Live typing respects PasteMethod::None / external-script delivery.

### Evaluated and dropped

- **DeepFilterNet**: the published `deep_filter` crate (0.2.5) ships no
  inference runtime (dataset/DSP only); the runtime exists only inside the
  DeepFilterNet training workspace as an unpublished git dependency. Not worth
  the supply-chain and build fragility while RNNoise + macOS Voice Isolation
  remain unbeaten in practice. Recorded in the PRD with the blocker.
- **Personal accent LoRA**: stays parked — the voice-setup wizard on the author's
  real voice confirmed Whisper Turbo as the winner, so the accuracy trigger
  (<90% sustained) is not met.

## v0.11.0 — 2026-07-02

### Added

- **Voice setup — in-app model benchmark**: Models → Voice setup walks you through
  reading three phrases (the third is built from your own custom words), then runs
  every downloaded model over the same recordings and scores accuracy (word-level
  WER, case/punctuation-insensitive) and speed (real-time factor). Speed breaks
  near-ties within 2% accuracy. One click applies the recommended model. Recorded
  audio stays in memory only — never history, never disk — and is discarded when
  the wizard closes; transcripts flow through the standard pipeline, which logs
  them to the local app log like every dictation. Includes a live mic-level bar and the same −40 dBFS too-quiet
  warning as dictation, so a bad capture can't silently poison the ranking.
- Benchmark phrases avoid numerals on purpose (models write "9:30" for
  "nine thirty"; word-level scoring would punish honest transcriptions).

### Changed

- Ollama documented as the preferred AI-edits provider (fully local and
  inspectable); UAT §5 updated with the working `qwen2.5:7b-instruct` recipe.

## v0.10.0 — 2026-07-02

The backlog release: every roadmap feature from the improvement plan, built and
unit-tested in one sweep (8 features, 114 lib tests passing).

### Added

- **Spoken punctuation** (default on): "period", "comma", "question mark", "new line" →
  symbols, deterministically — zero latency, no LLM. Article guards keep "the period"
  literal; sentence enders capitalize the next word. Toggle in General.
- **History search**: substring search over transcripts, post-processed text, and titles
  (parameterized SQL, wildcard-escaped). Debounced; clearing restores pagination.
- **Noise suppression** (default off, experimental): batch RNNoise pass over the
  transcription input (16k→48k→denoise→16k). History keeps the original audio.
  `--denoise` on the headless CLI for reproducible A/Bs. Toggle in Advanced.
- **Accent color**: preset swatches + custom color over the CSS-variable theme,
  #rrggbb-validated in Rust. Advanced settings.
- **Per-app prompts**: the frontmost app at transcription stop picks the cleanup prompt
  (via built-in lsappinfo — no new permission or dependency). Captured with a 3-second
  countdown in Post-processing settings.
- **Dictionary suggestions**: recurring mid-sentence capitalized words from your history
  appear as one-click chips beside custom words — confirm-to-promote, never automatic.
- **Command mode** (Ctrl+Shift+Space): copy text, hold, speak "make this formal" — the
  transformed text pastes. Local-only LLM, instruction-sandwich prompt defense, and on
  any failure nothing is pasted. New bindings merge additively into existing stores.
- **CI**: GitHub Actions validation (lint, translations, prettier, tsc, cargo fmt +
  lib tests on Ubuntu; advisory cargo/bun audits; manual macOS compile job — macOS
  runners bill 10x on the free tier).

## v0.9.2 — 2026-07-02

### Added

- **Mic-too-quiet warning**: when a recording's speech level (90th-percentile RMS of
  100 ms frames) is below −40 dBFS, a toast explains the transcript may be garbage and
  points at the input-volume setting. Quiet input makes every model hallucinate rather
  than fail — observed live at input volume 33/100. Level math is unit-tested,
  including the case where silence padding could mask a quiet voice.
- **Model activate/deactivate toggle**: a power button next to the model status dot
  loads or unloads the selected model without re-picking it from the dropdown —
  "which model is selected" and "is it in memory" are now separate, visible concerns.
- **Model pre-load at launch**: the selected model loads automatically on app start
  (skipped when the unload policy is "Immediately", which explicitly means on-demand),
  so the status dot starts green and the first dictation is instant.

### Fixed

- Upstream latent bug: `get_model_load_status` and `unload_model_manually` declared
  `State<TranscriptionManager>` while lib.rs manages `Arc<TranscriptionManager>` —
  they would have panicked on first use. Never hit upstream because nothing called
  them; the new toggle is their first caller.

## v0.9.1 — 2026-07-02 (first release as "HearMe")

### Changed

- **App renamed Handy → HearMe**: product name, bundle name (/Applications/HearMe.app),
  dmg name, menu-bar tooltip, window title, and all UI strings across 19 locales. The
  internal bundle identifier stays `com.pais.handy` deliberately — changing it would
  orphan settings, history, downloaded models, and permission grants. Upstream credits
  and URLs untouched. Versioning switches from `-local.N` suffixes to plain fork semver.

### Added

- **Model guidance on every model card**: "Best for / Not for" rows plus a note, so
  picking a model is a judgment call you can actually make from the UI. Guidance is
  editorial and fork-owned (`src/lib/modelGuidance.ts`), includes live A/B verdicts
  from this machine (e.g. Parakeet's weakness on West African English), and is
  searchable — typing "accent" in the models page finds the right models.
- Guidance labels localized across all 19 UI languages.

### Findings recorded

- **AfriSpeech whisper-medium fine-tune tested and REJECTED**: despite published
  benchmarks (+32% vs stock medium), on real Ghanaian-accented conversational speech
  it hallucinated wholesale while Whisper Turbo stayed near-verbatim. Benchmark
  domain (clinical read speech) did not transfer. Conversion procedure kept in
  docs/INSTALL.md spirit: convert-h5-to-ggml.py → drop .bin into the models folder.

## v0.9.0-local.1 — 2026-07-02

First release of the private fork. Base: upstream Handy v0.9.0 (commit f135970).

### Privacy hardening

- Update checks default **off** (stock Handy phones github.com on launch; an accepted
  auto-update would also overwrite this fork with an upstream build). Re-enablable in UI.
- Updater artifacts no longer produced at build time.
- **All cloud LLM post-processing providers removed** (OpenAI, Anthropic, Z.AI,
  OpenRouter, Groq, Cerebras, AWS Bedrock). Remaining: Apple Intelligence (on-device)
  and Custom, pre-pointed at localhost Ollama.
- Providers are **pruned on settings load** — a settings file from a previous upstream
  install cannot resurrect cloud providers or their API keys.
- Verified: zero network connections during dictation (`lsof -i` clean); the only
  network use is one-time SHA-256-pinned model downloads.

### Fixes

- Apple Intelligence bridge compiles with Command Line Tools only (removed the
  `@Generable` structured-output path that requires full Xcode's macro plugin; uses
  upstream's own plain-text fallback).

### Added

- **Hotkey doctor**: LaunchAgent that detects macOS version changes, probes whether the
  global shortcut still fires (synthetic keystroke + log verification), rebinds to a
  fallback combo only on confirmed failure, and escalates permission-class breakage to
  a notification + the right Settings pane. `scripts/install-hotkey-doctor.sh`.
- Fork docs: SPEC.md (decision record), UAT.md (verification checklist), docs/.

### Findings recorded

- Accent A/B on live Ghanaian-accented English (3 samples): **Whisper Turbo
  transcribes near-verbatim; Parakeet V3 drops 60–80% of words.** Turbo is also faster
  on M1 Pro for longer clips (Metal vs CPU ONNX). Default model set accordingly.
- Mic input volume below ~50/100 causes hallucinated transcripts on all models.
