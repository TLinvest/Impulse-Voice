#!/usr/bin/env bash
set -euo pipefail

readonly QS_ROOT="$HOME/.config/quickshell/ii"
readonly FAMILY_FILE="$QS_ROOT/panelFamilies/IllogicalImpulseFamily.qml"
readonly CUSTOM_KEYBINDS="$HOME/.config/hypr/custom/keybinds.conf"

systemctl --user disable --now impulse-voice.service 2>/dev/null || true

python3 - "$FAMILY_FILE" "$CUSTOM_KEYBINDS" <<'PY'
from pathlib import Path
import sys

family = Path(sys.argv[1])
if family.exists():
    content = family.read_text()
    content = content.replace("import qs.modules.ii.impulseVoice\n", "")
    content = content.replace("    PanelLoader { component: ImpulseVoice {} }\n", "")
    family.write_text(content)

keybinds = Path(sys.argv[2])
if keybinds.exists():
    content = keybinds.read_text()
    start = content.find("# BEGIN IMPULSE VOICE")
    end = content.find("# END IMPULSE VOICE")
    if start >= 0 and end >= 0:
        end += len("# END IMPULSE VOICE")
        content = (content[:start].rstrip() + "\n" + content[end:].lstrip()).rstrip() + "\n"
        keybinds.write_text(content)
PY

rm -f -- \
  "$HOME/.local/bin/impulse-voice-daemon" \
  "$HOME/.config/systemd/user/impulse-voice.service" \
  "$QS_ROOT/services/ImpulseVoiceService.qml" \
  "$QS_ROOT/modules/ii/impulseVoice/ImpulseVoice.qml"
rmdir "$QS_ROOT/modules/ii/impulseVoice" 2>/dev/null || true
systemctl --user daemon-reload
command -v hyprctl >/dev/null 2>&1 && hyprctl reload >/dev/null || true
[[ -f "$QS_ROOT/shell.qml" ]] && touch "$QS_ROOT/shell.qml"

echo "Impulse Voice was uninstalled. The local model was retained."
