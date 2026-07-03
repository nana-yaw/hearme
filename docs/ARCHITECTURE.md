# HearMe Architecture — the fork deltas and how to review them

Upstream architecture (managers, command-event bridge, pipeline, i18n rules, code style)
is documented in [AGENTS.md](../AGENTS.md) and remains accurate. This document covers what
the **fork** adds or changes, why, and how a human reviewer or AI agent should approach
the code. Release history lives in [CHANGELOG.md](CHANGELOG.md).

## The pipeline, with fork touch-points marked

```
⌥Space (rdev tap) ─▶ AudioRecordingManager ─▶ 16 kHz samples
                                                 │
                              ┌──────────────────┤
                              │  [FORK] speech_level_dbfs() < −40 dBFS?
                              │        └─▶ emit "recording-warning" (toast)
                              ▼
                       TranscriptionManager.transcribe()
                       (model kept warm; [FORK] pre-loaded at launch)
                              ▼
                       custom-words fuzzy correction
                              ▼
                 optional LLM post-process ([FORK] providers pruned to
                 Apple Intelligence + localhost/Ollama only)
                              ▼
                 clipboard save → ⌘V paste → restore
```

## Fork-delta map (file → what → why)

| File                                                                                                 | Change                                                                                                    | Why                                                                                                                              |
| ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/settings.rs`                                                                          | update checks default off; local-only provider defaults; **prune pass in `ensure_post_process_defaults`** | Privacy must survive pre-existing settings stores, not just fresh installs (HIGH security-review finding)                        |
| `src-tauri/tauri.conf.json`                                                                          | `createUpdaterArtifacts: false`; product name HearMe                                                      | No upstream signing key; fork never self-updates                                                                                 |
| `src-tauri/swift/apple_intelligence.swift`                                                           | `@Generable` structured path removed                                                                      | The macro plugin ships only with full Xcode; this machine is CLT-only. Upstream's own plain-text fallback is now the single path |
| `src-tauri/src/audio_toolkit/audio/utils.rs`                                                         | `speech_level_dbfs()` + unit tests                                                                        | Quiet input hallucinates instead of failing; 90th-percentile frame RMS so silence can't mask a quiet voice                       |
| `src-tauri/src/actions.rs`                                                                           | quiet check + `recording-warning` emission                                                                | Warn, don't block — the user decides                                                                                             |
| `src-tauri/src/commands/transcription.rs`                                                            | `load_model_manually`; **State types fixed to `Arc<TranscriptionManager>`**                               | Counterpart to unload; the upstream commands were latently broken (never called) and would panic — first real callers are ours   |
| `src-tauri/src/lib.rs`                                                                               | command registration; launch pre-load thread                                                              | Model dot starts green; "Immediately" policy respected                                                                           |
| `src/lib/modelGuidance.ts`                                                                           | fork-owned best-for/not-for editorial map                                                                 | Judgment on cards; kept out of the upstream-hot Rust catalog for merge cleanliness                                               |
| `src/components/onboarding/ModelCard.tsx`                                                            | guidance rows render                                                                                      | —                                                                                                                                |
| `src/components/model-selector/ModelSelector.tsx`                                                    | power toggle                                                                                              | Separates "selected" from "loaded"                                                                                               |
| `src/components/icons/HearMeTextLogo.tsx`, `src-tauri/icons/*`, `src-tauri/resources/tray_*.png`     | HearMe identity (wordmark, app icon, tray states)                                                         | Upstream's `HandyTextLogo`/`HandyHand` left in tree **unused** deliberately                                                      |
| `src/App.tsx`, `src/lib/types/events.ts`                                                             | `recording-warning` listener + type                                                                       | —                                                                                                                                |
| `src/i18n/locales/*`                                                                                 | brand rename + new keys in all 20 locales                                                                 | `check:translations` is a hard gate                                                                                              |
| `scripts/hotkey_doctor.py` + plist + installer                                                       | post-macOS-update verify→rebind→escalate agent                                                            | Deterministic Python; TCC-class breakage is human-only by OS design                                                              |
| `audio_toolkit/spoken_punctuation.rs` (+ hook in `managers/transcription.rs`)                        | "period"→"." with article guards, default on                                                              | Zero-latency alternative to LLM cleanup; unit-tested                                                                             |
| `audio_toolkit/denoise.rs` (+ `nnnoiseless` dep, actions.rs hook, `--denoise` CLI)                   | batch RNNoise pass, default off                                                                           | Transcription input only — History keeps original audio                                                                          |
| `managers/history.rs` search + `recent_transcripts`; `commands/history.rs`                           | history search + dictionary mining                                                                        | Parameterized SQL, LIKE-escaped                                                                                                  |
| `audio_toolkit/text.rs` `proper_noun_candidates`                                                     | recurring mid-sentence capitals → custom-word chips                                                       | Confirm-to-promote, never automatic                                                                                              |
| `app_context.rs`                                                                                     | frontmost bundle id via `lsappinfo`                                                                       | No new permission/dependency; feeds per-app prompt override                                                                      |
| `actions.rs` `post_process_with_prompt` + `command_mode_transform` + `TranscribeAction.command_mode` | command mode on copied text                                                                               | Fails closed: on any failure nothing is pasted                                                                                   |
| `settings.rs` `ensure_default_bindings` + `command_mode` binding                                     | new bindings reach old stores additively                                                                  | Never touches customized combos                                                                                                  |
| `shortcut/mod.rs` accent/noise/punctuation/profile commands; `AccentColor.tsx` etc.                  | settings surface for the v0.10.0 features                                                                 | Accent color #rrggbb-validated before the CSS property                                                                           |
| `benchmark.rs` (+ tests), `commands/benchmark.rs`                                                    | voice-setup benchmark: WER + recommendation, recording/orchestration commands                             | Model choice becomes a measurement of the user's own voice; audio held in memory only, never history/disk                        |
| `VoiceSetupWizard.tsx`, `lib/benchmarkPhrases.ts`, Models page button                                | wizard UI; English stimuli stay in code (WER reference must equal displayed text)                         | Phrases avoid numerals — "nine thirty" vs "9:30" would corrupt word-level scoring                                                |
| `selected_text.rs` (macOS AX FFI), `choose_command_source` in `actions.rs`                           | command mode prefers the live selection over the clipboard                                                | Same Accessibility grant the paste path already needs; every AX failure falls back to clipboard                                  |
| `live_typing.rs` (+ hooks in `managers/transcription.rs` stream worker)                              | experimental type-while-speaking with erase-then-paste reconciliation                                     | Erase completes before the finalize reply, so the paste path stays byte-identical; focus change halts typing                     |
| `.github/workflows/validate.yml`                                                                     | CI validation                                                                                             | Ubuntu always-on; macOS compile manual (10x billing)                                                                             |
| `SPEC.md`, `UAT.md`, `docs/*`                                                                        | fork documentation set                                                                                    | —                                                                                                                                |

## Invariants a reviewer should protect

1. **No new network paths.** `reqwest` exists only in `llm_client.rs` (localhost/on-device
   after pruning) and `managers/model.rs` (SHA-256-pinned one-time downloads). Any diff
   adding an outbound call is a privacy regression until proven otherwise.
2. **The prune must stay on the load path.** `ensure_post_process_defaults` runs on every
   settings load; weakening it re-opens cloud egress for upgraded stores.
3. **State params name the exact managed type.** lib.rs manages `Arc<Manager>`; a command
   declaring `State<Manager>` compiles and then panics at first invocation.
4. **i18n completeness is a build gate.** New user-facing strings need all 20 locales.
5. **Upstream files are parked, not deleted.** Unused ≠ dead in a fork; deletions maximize
   merge conflicts for zero behavior.
6. **Binary name stays `handy`** (crate name) even though the bundle is HearMe.app — the
   hotkey doctor's `pgrep -x handy` and the headless CLI path depend on it.
7. **Model claims require the recorded-sample benchmark**, not leaderboard numbers — the
   AfriSpeech rejection is the precedent (docs/CHANGELOG.md v0.9.2).

## Known sharp edges (cost you real time if unknown)

- **TCC grants die on every rebuild** (ad-hoc signing → new CDHash). Batch UI changes;
  re-grant after installs; `tccutil reset All com.pais.handy` un-sticks lying toggles.
- **Moving/renaming the repo directory poisons the cargo cache**: build scripts bake
  absolute paths into OUT_DIR artifacts (`ferrous-opencc`, `tauri` + plugins permission
  files). Symptom: "No such file … /old/path/…". Fix: `cargo clean -p <crate> --release`
  for each offender — not a full clean.
- **A gitconfig that rewrites GitHub HTTPS→SSH** breaks cargo's libgit2 agent auth
  here. `CARGO_NET_GIT_FETCH_WITH_CLI=true` is mandatory (in docs/INSTALL.md recipe).
- **`bindings.ts` is generated** (tauri-specta) but hand-extended in this fork for
  `loadModelManually` following the exact generated shape — a dev-mode run will
  regenerate it identically; don't "fix" the style.
- **Model status in the UI is event-driven** (`model-state-changed`); the settings window
  snapshots the model list at mount — a model downloaded externally appears after a
  rescan or restart (this bit us live: "No models match this filter").

## How to verify a change (fastest honest loop)

```bash
# Rust logic:      cargo test --release --lib <filter>
# Rust compiles:   cargo check --release        (shares the release cache)
# Frontend gates:  bun run lint && bun run check:translations && bun run build
# Model claims:    /Applications/HearMe.app/Contents/MacOS/handy \
#                    --transcribe-file sample.wav --model <id> --json
# Privacy claims:  lsof -i -a -p $(pgrep -x handy)   # while dictating
# Full ship:       bun run tauri build → install → re-grant TCC → UAT.md
```
