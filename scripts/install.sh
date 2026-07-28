#!/usr/bin/env bash
set -euo pipefail

download_model=true
integrate_quickshell=true
start_service=true

for argument in "$@"; do
  case "$argument" in
    --no-model) download_model=false ;;
    --no-quickshell) integrate_quickshell=false ;;
    --no-start) start_service=false ;;
    -h|--help)
      cat <<'EOF'
Usage: ./scripts/install.sh [--no-model] [--no-quickshell] [--no-start]

Build and install Impulse Voice for the current user.
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $argument" >&2
      exit 2
      ;;
  esac
done

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
readonly QS_ROOT="$HOME/.config/quickshell/ii"
readonly FAMILY_FILE="$QS_ROOT/panelFamilies/IllogicalImpulseFamily.qml"
readonly CUSTOM_KEYBINDS="$HOME/.config/hypr/custom/keybinds.conf"
quickshell_changed=false
hyprland_changed=false

for command in cargo install systemctl python3; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "Required command not found: $command" >&2
    exit 1
  }
done

if "$download_model"; then
  "$SCRIPT_DIR/download-model.sh"
fi

echo "Building release binary…"
cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
install -Dm755 \
  "$REPO_ROOT/target/release/impulse-voice-daemon" \
  "$HOME/.local/bin/impulse-voice-daemon"
install -Dm644 \
  "$REPO_ROOT/systemd/impulse-voice.service" \
  "$HOME/.config/systemd/user/impulse-voice.service"

if "$integrate_quickshell"; then
  [[ -d "$QS_ROOT" ]] || {
    echo "Illogical Impulse configuration not found: $QS_ROOT" >&2
    exit 1
  }
  [[ -f "$FAMILY_FILE" ]] || {
    echo "Illogical Impulse panel family not found: $FAMILY_FILE" >&2
    exit 1
  }

  if ! cmp -s \
    "$REPO_ROOT/quickshell/services/ImpulseVoiceService.qml" \
    "$QS_ROOT/services/ImpulseVoiceService.qml"; then
    install -Dm644 \
      "$REPO_ROOT/quickshell/services/ImpulseVoiceService.qml" \
      "$QS_ROOT/services/ImpulseVoiceService.qml"
    quickshell_changed=true
  fi
  if ! cmp -s \
    "$REPO_ROOT/quickshell/modules/ii/impulseVoice/ImpulseVoice.qml" \
    "$QS_ROOT/modules/ii/impulseVoice/ImpulseVoice.qml"; then
    install -Dm644 \
      "$REPO_ROOT/quickshell/modules/ii/impulseVoice/ImpulseVoice.qml" \
      "$QS_ROOT/modules/ii/impulseVoice/ImpulseVoice.qml"
    quickshell_changed=true
  fi

  family_changed="$(python3 - "$FAMILY_FILE" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
original = content

import_line = "import qs.modules.ii.impulseVoice"
if import_line not in content:
    anchor = "import qs.modules.ii.wallpaperSelector"
    if anchor not in content:
        raise SystemExit(f"Import anchor not found in {path}")
    content = content.replace(anchor, f"{anchor}\n{import_line}", 1)

loader_line = "    PanelLoader { component: ImpulseVoice {} }"
if loader_line not in content:
    anchor = "    PanelLoader { component: WallpaperSelector {} }"
    if anchor not in content:
        raise SystemExit(f"Panel loader anchor not found in {path}")
    content = content.replace(anchor, f"{anchor}\n{loader_line}", 1)

if content != original:
    backup = path.with_suffix(path.suffix + ".pre-impulse-voice")
    if not backup.exists():
        backup.write_text(original)
    path.write_text(content)
    print("true")
else:
    print("false")
PY
)"
  if [[ "$family_changed" == true ]]; then
    quickshell_changed=true
  fi

  mkdir -p "$(dirname "$CUSTOM_KEYBINDS")"
  touch "$CUSTOM_KEYBINDS"
  keybinds_changed="$(python3 - "$CUSTOM_KEYBINDS" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
original = content
begin = "# BEGIN IMPULSE VOICE"
end = "# END IMPULSE VOICE"
block = """# BEGIN IMPULSE VOICE
# Hold Super+Alt+V to dictate, then release to transcribe and insert.
bindd = Super+Alt, V, Hold Impulse Voice dictation, global, quickshell:impulseVoiceHold
bindd = Super+Alt+Shift, V, Toggle Impulse Voice, global, quickshell:impulseVoiceToggle
bindd = Super+Alt, Escape, Cancel Impulse Voice, global, quickshell:impulseVoiceCancel
# END IMPULSE VOICE"""

if begin in content:
    start = content.index(begin)
    finish = content.find(end, start)
    if finish < 0:
        raise SystemExit(f"Incomplete Impulse Voice block in {path}")
    finish += len(end)
    content = content[:start] + block + content[finish:]
else:
    content = content.rstrip() + "\n\n" + block + "\n"

if content != original:
    path.write_text(content)
    print("true")
else:
    print("false")
PY
)"
  if [[ "$keybinds_changed" == true ]]; then
    hyprland_changed=true
  fi
fi

systemctl --user daemon-reload
if "$start_service"; then
  systemctl --user enable impulse-voice.service
  systemctl --user restart impulse-voice.service
fi

if "$integrate_quickshell"; then
  if "$hyprland_changed" && command -v hyprctl >/dev/null 2>&1; then
    hyprctl reload >/dev/null || true
  fi
  if "$quickshell_changed"; then
    # Illogical Impulse checks for kded6 on every shell reload. Only reload when
    # the installed QML actually changed, otherwise its conflict dialog appears
    # on every harmless reinstall.
    touch "$QS_ROOT/shell.qml"
  fi
fi

echo
"$HOME/.local/bin/impulse-voice-daemon" --doctor
echo
echo "Installation complete."
echo "Hold Super+Alt+V, speak, then release to transcribe."
