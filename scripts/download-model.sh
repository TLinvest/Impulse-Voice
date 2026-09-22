#!/usr/bin/env bash
set -euo pipefail
readonly REVISION=ab9eb5ef7b81f98211b3feb68e5a856cab71f913
readonly WEIGHTS_SHA256=78ec25733ee0d0c1586d1346fc86db9d0c2e436e3a8ab1d32a82d1bb8f848d21
readonly DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
readonly MODEL_ROOT="${IMPULSE_VOICE_MODEL_ROOT:-$DATA_HOME/impulse-voice/models}"
readonly MODEL_DIR="$MODEL_ROOT/parakeet-redux"
readonly FILES=(model.safetensors config.json tokenizer.json ternary.json README.md)
if [[ -f "$MODEL_DIR/.revision" ]] && [[ "$(cat "$MODEL_DIR/.revision")" == "$REVISION" ]]; then
  complete=true
  for file in "${FILES[@]}"; do [[ -s "$MODEL_DIR/$file" ]] || complete=false; done
  if "$complete" && echo "$WEIGHTS_SHA256  $MODEL_DIR/model.safetensors" | sha256sum --check --status; then
    echo "Parakeet Redux is already installed at $MODEL_DIR"
    exit 0
  fi
fi
[[ ! -e "$MODEL_DIR" ]] || { echo "Incomplete or different model at $MODEL_DIR; move it before retrying." >&2; exit 1; }
mkdir -p "$MODEL_ROOT"
staging="$(mktemp -d "$MODEL_ROOT/.redux-download.XXXXXX")"
trap 'rm -rf -- "$staging"' EXIT
for file in "${FILES[@]}"; do
  curl --fail --location --retry 3 --output "$staging/$file" \
    "https://huggingface.co/moondream/parakeet-redux/resolve/$REVISION/$file"
done
echo "$WEIGHTS_SHA256  $staging/model.safetensors" | sha256sum --check -
printf '%s\n' "$REVISION" > "$staging/.revision"
mv "$staging" "$MODEL_DIR"
echo "Parakeet Redux installed at $MODEL_DIR"
