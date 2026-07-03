#!/bin/bash
# Install the hotkey-doctor LaunchAgent (idempotent — safe to re-run).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LABEL="com.hearme.hotkey-doctor"
PLIST_SRC="$SCRIPT_DIR/$LABEL.plist"
PLIST_DST="$HOME/Library/LaunchAgents/$LABEL.plist"

mkdir -p "$HOME/Library/LaunchAgents" "$HOME/Library/Logs/com.pais.handy"
sed -e "s|__SCRIPT_PATH__|$SCRIPT_DIR/hotkey_doctor.py|" \
    -e "s|__HOME__|$HOME|g" \
    "$PLIST_SRC" > "$PLIST_DST"
plutil -lint "$PLIST_DST"

launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$PLIST_DST"
launchctl print "gui/$(id -u)/$LABEL" | head -3

echo "Installed. Prime the automation permission with one manual probe:"
echo "  python3 $SCRIPT_DIR/hotkey_doctor.py --force --no-patch"
