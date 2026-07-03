# Installing on a new Mac

Two paths: grab the prebuilt release (5 minutes) or build from source (~30 minutes).

## Path A — install the release (recommended)

1. Go to this repo's **Releases** page and download `HearMe_x.y.z_aarch64.dmg`
   (Apple Silicon only — this fork is built and tested on arm64).
2. Open the dmg, drag **HearMe** to Applications.
3. First launch: the app is self-signed (no Apple Developer certificate), so macOS
   Gatekeeper will block a normal double-click. Either **right-click → Open → Open**,
   or run:
   ```bash
   xattr -dr com.apple.quarantine /Applications/HearMe.app
   ```
4. Follow onboarding and grant the three permissions (all required):
   - **Microphone** — capture your speech
   - **Accessibility** — paste the transcript into the frontmost app
   - **Input Monitoring** — the global push-to-talk hotkey
5. Pick a transcription model. It downloads once (SHA-256-verified), then everything
   runs offline. Recommended for West African / Ghanaian accented English:
   **Whisper Turbo** (verified dramatically better than Parakeet on this — see
   [FEATURES.md](FEATURES.md)). The AfriSpeech accent-tuned model, if present in the
   release assets, goes into `~/Library/Application Support/com.pais.handy/models/`
   and appears in the picker automatically.
6. Test: open Notes, **hold Option+Space**, speak, release.

### Optional extras

- **Hotkey doctor** (self-heals the shortcut after macOS updates):
  ```bash
  git clone <this-repo> ~/Projects/hearme
  ~/Projects/hearme/scripts/install-hotkey-doctor.sh
  python3 ~/Projects/hearme/scripts/hotkey_doctor.py --force --no-patch  # primes permission
  ```
- **Custom words** (proper nouns the models miss): Settings → add names like
  Kwame, Adjoa, Kumasi, Accra, cedis.
- **AI cleanup** (fillers, spoken punctuation — off by default, fully local):
  Settings → Post-processing → provider **Apple Intelligence**, or install Ollama
  (`brew install ollama && ollama pull qwen2.5:7b-instruct`) and use **Custom**
  (pre-pointed at `http://localhost:11434/v1`).
- **Mic level**: input volume below ~50 makes every model hallucinate. Check
  System Settings → Sound → Input, speak at normal laptop distance.

## Path B — build from source

Prerequisites: Homebrew, then:

```bash
brew install rustup bun cmake ffmpeg
rustup default stable
git clone <this-repo> ~/Projects/hearme && cd ~/Projects/hearme
bun install
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

Build (the env vars matter — see notes):

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
export CMAKE_POLICY_VERSION_MINIMUM=3.5
export CARGO_NET_GIT_FETCH_WITH_CLI=true   # needed if your gitconfig rewrites https→ssh
bun run tauri build
```

Output: `src-tauri/target/release/bundle/macos/HearMe.app` and a dmg under
`bundle/dmg/`. Copy the .app to /Applications.

Notes:

- Works with Xcode **Command Line Tools only** — this fork removed the `@Generable`
  macro dependency that otherwise requires full Xcode.
- The build is ad-hoc signed: **every rebuild invalidates the TCC permission grants.**
  Re-grant in System Settings, or `tccutil reset All com.pais.handy` first.
- Upstream base: [cjpais/Handy](https://github.com/cjpais/Handy) (`upstream` remote).
  See [SPEC.md](../SPEC.md) for every fork change and why.
