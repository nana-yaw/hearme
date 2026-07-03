#!/usr/bin/env python3
"""Hotkey doctor: verify-then-heal Handy's global shortcut after a macOS update.

macOS updates have twice broken Handy's hotkey capture (upstream issue #1578).
The known user-level fix is rebinding to a different combo — but only when the
event tap is actually broken; rebinding blindly after every update would churn
the user's chosen shortcut. So this script:

  1. Runs at login (and when SystemVersion.plist changes) via a LaunchAgent,
     and exits immediately unless the macOS version differs from the last run.
  2. PROBES: synthesizes the currently-configured shortcut keystroke (event
     taps see synthetic events) and watches Handy's debug log for
     "Recording started for binding" — ground truth that the tap fired.
  3. HEALS: on a failed probe, rewrites the binding in settings_store.json to
     FALLBACK_BINDING, relaunches Handy, and re-probes.
  4. ESCALATES: if the fallback also fails, the cause is almost certainly a
     TCC permission invalidation, which macOS forbids fixing programmatically.
     The original binding is restored and a notification deep-links the
     Privacy & Security pane for the one fix only a human can perform.

Flags: --force (probe even without a version change), --no-patch (probe and
report only). Run once manually with --force to grant the osascript/System
Events automation permission the probe needs.
"""

import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

APP_SUPPORT = Path.home() / "Library/Application Support/com.pais.handy"
SETTINGS_STORE = APP_SUPPORT / "settings_store.json"
STATE_FILE = APP_SUPPORT / "hotkey-doctor-state.json"
HANDY_LOG = Path.home() / "Library/Logs/com.pais.handy/handy.log"
DOCTOR_LOG = Path.home() / "Library/Logs/com.pais.handy/hotkey-doctor.log"
_INSTALLED_APP = Path("/Applications/HearMe.app")
_BUILD_APP = Path.home() / "Projects/hearme/src-tauri/target/release/bundle/macos/HearMe.app"
APP_BUNDLE = _INSTALLED_APP if _INSTALLED_APP.exists() else _BUILD_APP
HANDY_BINARY = APP_BUNDLE / "Contents/MacOS/handy"

BINDING_ID = "transcribe"
FALLBACK_BINDING = "ctrl+option+space"
PROBE_SIGNAL = "Recording started for binding"
PROBE_WAIT_SECONDS = 2.5
APP_START_WAIT_SECONDS = 8

# AppleScript "key code" values for non-character keys; letters use keystroke.
KEY_CODES = {"space": 49, "escape": 53, "tab": 48, "return": 36}
MODIFIER_MAP = {
    "ctrl": "control down",
    "control": "control down",
    "option": "option down",
    "alt": "option down",
    "shift": "shift down",
    "cmd": "command down",
    "command": "command down",
    "super": "command down",
}


def log(message: str) -> None:
    DOCTOR_LOG.parent.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y-%m-%d %H:%M:%S")
    with DOCTOR_LOG.open("a") as handle:
        handle.write(f"[{stamp}] {message}\n")
    print(message)


def notify(title: str, body: str) -> None:
    script = f'display notification "{body}" with title "{title}"'
    subprocess.run(["osascript", "-e", script], check=False)


def macos_version() -> str:
    result = subprocess.run(
        ["sw_vers", "-productVersion"], capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


def read_state() -> dict:
    if STATE_FILE.exists():
        try:
            return json.loads(STATE_FILE.read_text())
        except (json.JSONDecodeError, OSError):
            pass
    return {}


def write_state(state: dict) -> None:
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    STATE_FILE.write_text(json.dumps(state, indent=2))


def current_binding() -> str:
    settings = json.loads(SETTINGS_STORE.read_text())
    return settings["settings"]["bindings"][BINDING_ID]["current_binding"]


def set_binding(binding: str) -> None:
    store = json.loads(SETTINGS_STORE.read_text())
    store["settings"]["bindings"][BINDING_ID]["current_binding"] = binding
    SETTINGS_STORE.write_text(json.dumps(store, indent=2))


def handy_running() -> bool:
    return subprocess.run(["pgrep", "-x", "handy"], capture_output=True).returncode == 0


def quit_handy() -> None:
    subprocess.run(["pkill", "-x", "handy"], check=False)
    for _ in range(10):
        if not handy_running():
            return
        time.sleep(0.5)


def launch_handy() -> bool:
    subprocess.run(["open", "-a", str(APP_BUNDLE), "--args", "--start-hidden"], check=False)
    deadline = time.time() + APP_START_WAIT_SECONDS
    while time.time() < deadline:
        if handy_running():
            time.sleep(2)  # give the shortcut listener time to register
            return True
        time.sleep(0.5)
    return False


def synthesize_shortcut(binding: str) -> bool:
    """Post the configured combo via System Events. Returns False if the
    binding uses a key this probe cannot synthesize."""
    tokens = [token.strip().lower() for token in binding.split("+") if token.strip()]
    modifiers = [MODIFIER_MAP[t] for t in tokens if t in MODIFIER_MAP]
    keys = [t for t in tokens if t not in MODIFIER_MAP]
    if len(keys) != 1:
        return False
    key = keys[0]
    using = f" using {{{', '.join(modifiers)}}}" if modifiers else ""
    if key in KEY_CODES:
        action = f"key code {KEY_CODES[key]}{using}"
    elif len(key) == 1 and re.fullmatch(r"[a-z0-9]", key):
        action = f'keystroke "{key}"{using}'
    else:
        return False
    script = f'tell application "System Events" to {action}'
    result = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
    if result.returncode != 0:
        log(f"osascript failed (likely missing Automation permission): {result.stderr.strip()}")
        return False
    return True


def probe(binding: str) -> bool:
    """True if synthesizing the binding makes Handy start a recording."""
    log_offset = HANDY_LOG.stat().st_size if HANDY_LOG.exists() else 0
    if not synthesize_shortcut(binding):
        return False
    time.sleep(PROBE_WAIT_SECONDS)
    # Cancel the recording the probe just started (harmless if none started).
    subprocess.run([str(HANDY_BINARY), "--cancel"], check=False, capture_output=True)
    if not HANDY_LOG.exists():
        return False
    with HANDY_LOG.open() as handle:
        handle.seek(log_offset)
        return PROBE_SIGNAL in handle.read()


def open_privacy_pane() -> None:
    subprocess.run(
        ["open", "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"],
        check=False,
    )


def main() -> int:
    force = "--force" in sys.argv
    no_patch = "--no-patch" in sys.argv

    state = read_state()
    version = macos_version()
    if not force and state.get("last_macos_version") == version:
        return 0  # No macOS update since last check — nothing to do.

    log(f"macOS {state.get('last_macos_version', 'unknown')} -> {version}; probing hotkey")

    if not SETTINGS_STORE.exists():
        log("No Handy settings store yet (app never ran) — skipping until first run")
        write_state({"last_macos_version": version, "last_result": "skipped-no-settings"})
        return 0

    if not handy_running() and not launch_handy():
        log("Handy failed to launch — cannot probe")
        notify("Handy hotkey doctor", "Handy did not launch after the macOS update.")
        write_state({"last_macos_version": version, "last_result": "app-launch-failed"})
        return 1

    binding = current_binding()
    if probe(binding):
        log(f"Hotkey '{binding}' verified working — no action needed")
        write_state({"last_macos_version": version, "last_result": "healthy"})
        return 0

    log(f"Hotkey '{binding}' did NOT fire")
    if no_patch:
        write_state({"last_macos_version": version, "last_result": "broken-no-patch"})
        return 1

    # Heal: rebind to the fallback combo and re-probe.
    log(f"Rebinding {BINDING_ID} to '{FALLBACK_BINDING}' and relaunching")
    quit_handy()
    shutil.copy2(SETTINGS_STORE, SETTINGS_STORE.with_suffix(".json.doctor-backup"))
    set_binding(FALLBACK_BINDING)
    if not launch_handy():
        log("Handy failed to relaunch after rebind")
        write_state({"last_macos_version": version, "last_result": "relaunch-failed"})
        return 1

    if probe(FALLBACK_BINDING):
        log(f"Fallback '{FALLBACK_BINDING}' works — rebind kept")
        notify(
            "Handy hotkey rebound",
            f"The macOS update broke '{binding}'. Dictation now uses {FALLBACK_BINDING}.",
        )
        write_state({"last_macos_version": version, "last_result": "rebound"})
        return 0

    # Both combos dead: this is the permission class of breakage — only a human
    # can re-grant TCC permissions. Restore the user's original binding.
    log("Fallback also failed — restoring original binding; likely TCC invalidation")
    quit_handy()
    set_binding(binding)
    launch_handy()
    notify(
        "Handy needs permissions re-granted",
        "The macOS update likely reset Accessibility/Input Monitoring for Handy. Opening the Settings pane.",
    )
    open_privacy_pane()
    write_state({"last_macos_version": version, "last_result": "needs-human-permissions"})
    return 1


if __name__ == "__main__":
    sys.exit(main())
