#!/usr/bin/env bash
set -euo pipefail
readonly DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
readonly VENV="$DATA_HOME/impulse-voice/photon-venv"
command -v uv >/dev/null || { echo 'Install uv before running this installer.' >&2; exit 1; }
[[ -x "$VENV/bin/python" ]] || uv venv "$VENV" --python 3.12
uv pip install --python "$VENV/bin/python" 'moondream==2.4.0' 'torch==2.14.0+cpu' \
  --index https://download.pytorch.org/whl/cpu --index-strategy unsafe-best-match
