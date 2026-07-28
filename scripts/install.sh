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

Compile et installe Impulse Voice pour l'utilisateur courant.
EOF
      exit 0
      ;;
    *)
      echo "Option inconnue : $argument" >&2
      exit 2
      ;;
  esac
done

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
readonly QS_ROOT="$HOME/.config/quickshell/ii"
readonly FAMILY_FILE="$QS_ROOT/panelFamilies/IllogicalImpulseFamily.qml"
readonly CUSTOM_KEYBINDS="$HOME/.config/hypr/custom/keybinds.conf"

for command in cargo install systemctl python3; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "Commande requise introuvable : $command" >&2
    exit 1
  }
done

if "$download_model"; then
  "$SCRIPT_DIR/download-model.sh"
fi

echo "Compilation release…"
cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
install -Dm755 \
  "$REPO_ROOT/target/release/impulse-voice-daemon" \
  "$HOME/.local/bin/impulse-voice-daemon"
install -Dm644 \
  "$REPO_ROOT/systemd/impulse-voice.service" \
  "$HOME/.config/systemd/user/impulse-voice.service"

if "$integrate_quickshell"; then
  [[ -d "$QS_ROOT" ]] || {
    echo "Configuration Illogical Impulse introuvable : $QS_ROOT" >&2
    exit 1
  }
  [[ -f "$FAMILY_FILE" ]] || {
    echo "Panel family introuvable : $FAMILY_FILE" >&2
    exit 1
  }

  install -Dm644 \
    "$REPO_ROOT/quickshell/services/ImpulseVoiceService.qml" \
    "$QS_ROOT/services/ImpulseVoiceService.qml"
  install -Dm644 \
    "$REPO_ROOT/quickshell/modules/ii/impulseVoice/ImpulseVoice.qml" \
    "$QS_ROOT/modules/ii/impulseVoice/ImpulseVoice.qml"

  python3 - "$FAMILY_FILE" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
content = path.read_text()
original = content

import_line = "import qs.modules.ii.impulseVoice"
if import_line not in content:
    anchor = "import qs.modules.ii.wallpaperSelector"
    if anchor not in content:
        raise SystemExit(f"Point d'import introuvable dans {path}")
    content = content.replace(anchor, f"{anchor}\n{import_line}", 1)

loader_line = "    PanelLoader { component: ImpulseVoice {} }"
if loader_line not in content:
    anchor = "    PanelLoader { component: WallpaperSelector {} }"
    if anchor not in content:
        raise SystemExit(f"Point de chargement introuvable dans {path}")
    content = content.replace(anchor, f"{anchor}\n{loader_line}", 1)

if content != original:
    backup = path.with_suffix(path.suffix + ".pre-impulse-voice")
    if not backup.exists():
        backup.write_text(original)
    path.write_text(content)
PY

  mkdir -p "$(dirname "$CUSTOM_KEYBINDS")"
  touch "$CUSTOM_KEYBINDS"
  if ! grep -q 'BEGIN IMPULSE VOICE' "$CUSTOM_KEYBINDS"; then
    cat >>"$CUSTOM_KEYBINDS" <<'EOF'

# BEGIN IMPULSE VOICE
# Maintenir Super+Alt+V pour dicter, relâcher pour transcrire et coller.
bindd = Super+Alt, V, Start Impulse Voice dictation, global, quickshell:impulseVoiceStart
bindrd = Super+Alt, V, Stop Impulse Voice dictation, global, quickshell:impulseVoiceStop
bindd = Super+Alt+Shift, V, Toggle Impulse Voice, global, quickshell:impulseVoiceToggle
bindd = Super+Alt, Escape, Cancel Impulse Voice, global, quickshell:impulseVoiceCancel
# END IMPULSE VOICE
EOF
  fi
fi

systemctl --user daemon-reload
if "$start_service"; then
  systemctl --user enable impulse-voice.service
  systemctl --user restart impulse-voice.service
fi

if "$integrate_quickshell"; then
  command -v hyprctl >/dev/null 2>&1 && hyprctl reload >/dev/null || true
  touch "$QS_ROOT/shell.qml"
fi

echo
"$HOME/.local/bin/impulse-voice-daemon" --doctor
echo
echo "Installation terminée."
echo "Maintiens Super+Alt+V, parle, puis relâche pour transcrire."
