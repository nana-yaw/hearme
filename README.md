# HearMe

![Downloads](https://img.shields.io/github/downloads/nana-yaw/hearme/total) ![License](https://img.shields.io/github/license/nana-yaw/hearme)

**Private dictation that finds the model that hears _your_ voice best — and never sends a byte anywhere.**

🌍 **[hearme — one-page site & download](https://nana-yaw.github.io/hearme/)**

HearMe is a hardened, feature-extended fork of the excellent [Handy](https://github.com/cjpais/Handy) by CJ Pais. Hold a hotkey anywhere on your desktop, speak, release — the transcript is typed at your cursor. Everything runs on-device.

## Why this fork exists

Mainstream speech models are benchmarked on American and European voices. On West-African-accented English, the model topping the public leaderboards silently dropped **60–80% of the words** in live tests, while another transcribed near-verbatim. You can't see that on a leaderboard — so HearMe measures it: the **voice-setup wizard** has you read three short phrases once, scores every model on your machine against _your_ voice (word error rate + speed), and applies the winner in one click.

## What the fork adds over upstream

- **Voice-setup wizard** — benchmark all downloaded models on your own voice, apply the best
- **Local-only by construction** — cloud LLM providers are removed from the code (not just disabled) and pruned from old settings on load; update phone-home off by default
- **Command mode** — highlight text, hold `Ctrl+Shift+Space`, say "make this more formal": a local LLM (Ollama) rewrites it in place, reading the selection via the Accessibility API
- **Live typing** (experimental) — words appear while you speak; the cleaned text re-settles on release
- **Spoken punctuation** — "period" → "." deterministically, no LLM latency
- **Dictionary suggestions** — recurring names from your history become one-click custom words
- **Per-app cleanup prompts, history search, mic-too-quiet warning, model activate toggle + launch preload, accent color, self-healing hotkey doctor**

Full list: [docs/FEATURES.md](docs/FEATURES.md) · release history: [docs/CHANGELOG.md](docs/CHANGELOG.md) · design/internals: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Install

**macOS** (Apple Silicon, the tested platform): grab the `.dmg` from [Releases](https://github.com/nana-yaw/hearme/releases/latest), drag to Applications. The build is unsigned: **right-click → Open** on first launch, then grant Microphone, Accessibility, and Input Monitoring when prompted.

**Windows (beta)**: `..._x64-setup.exe` (Intel/AMD) or `..._arm64-setup.exe` (ARM). Unsigned — SmartScreen will warn: "More info" → "Run anyway".

**Linux (beta)**: `.deb`, `.rpm`, or `.AppImage` from the same release, in `amd64` and `arm64` flavors. Wayland support is limited upstream.

Windows and Linux builds are CI-built from the same source but not yet hand-tested on real hardware — [issues](https://github.com/nana-yaw/hearme/issues) welcome. Building from source: [docs/INSTALL.md](docs/INSTALL.md).

## Don't trust — verify

```bash
lsof -i -a -p $(pgrep -x handy)   # while dictating: zero rows
```

The only network use is the one-time, SHA-256-pinned model download you trigger yourself.

## Credits

- Forked from [cjpais/Handy](https://github.com/cjpais/Handy) (MIT) — thank you for the outstanding foundation.
- Built end-to-end with **Claude Fable 5** (Anthropic): research, code, adversarial reviews, docs, and the landing page.

## License

[MIT](LICENSE), same as upstream.
