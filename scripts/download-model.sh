#!/usr/bin/env bash
set -euo pipefail

readonly MODEL_URL="https://blob.handy.computer/parakeet-v3-int8.tar.gz"
readonly MODEL_SHA256="43d37191602727524a7d8c6da0eef11c4ba24320f5b4730f1a2497befc2efa77"
readonly MODEL_NAME="parakeet-tdt-0.6b-v3-int8"
readonly DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
readonly MODEL_ROOT="${IMPULSE_VOICE_MODEL_ROOT:-$DATA_HOME/impulse-voice/models}"
readonly MODEL_DIR="$MODEL_ROOT/$MODEL_NAME"
readonly ARCHIVE="$MODEL_ROOT/$MODEL_NAME.tar.gz.part"
readonly REQUIRED_FILES=(
  "encoder-model.int8.onnx"
  "decoder_joint-model.int8.onnx"
  "nemo128.onnx"
  "vocab.txt"
)

model_is_complete() {
  local file
  for file in "${REQUIRED_FILES[@]}"; do
    [[ -f "$MODEL_DIR/$file" ]] || return 1
  done
}

if model_is_complete; then
  echo "Parakeet V3 INT8 is already installed at $MODEL_DIR"
  exit 0
fi

if [[ -e "$MODEL_DIR" ]]; then
  echo "The model directory exists but is incomplete: $MODEL_DIR" >&2
  echo "Move or remove it explicitly before retrying the download." >&2
  exit 1
fi

for command in curl sha256sum tar find mktemp; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "Required command not found: $command" >&2
    exit 1
  }
done

mkdir -p "$MODEL_ROOT"
echo "Downloading Parakeet V3 INT8 (~478 MB)…"
curl \
  --fail \
  --location \
  --retry 3 \
  --continue-at - \
  --output "$ARCHIVE" \
  "$MODEL_URL"

echo "$MODEL_SHA256  $ARCHIVE" | sha256sum --check -

extract_dir="$(mktemp -d "$MODEL_ROOT/.parakeet-extract.XXXXXX")"
cleanup() {
  rm -rf -- "$extract_dir"
}
trap cleanup EXIT

tar -xzf "$ARCHIVE" -C "$extract_dir"
encoder_path="$(find "$extract_dir" -type f -name 'encoder-model.int8.onnx' -print -quit)"
if [[ -z "$encoder_path" ]]; then
  echo "Invalid archive: encoder-model.int8.onnx is missing." >&2
  exit 1
fi

source_dir="$(dirname "$encoder_path")"
for file in "${REQUIRED_FILES[@]}"; do
  [[ -f "$source_dir/$file" ]] || {
    echo "Invalid archive: $file is missing." >&2
    exit 1
  }
done

mkdir -p "$MODEL_DIR"
cp -a "$source_dir/." "$MODEL_DIR/"
find "$MODEL_DIR" -type f -name '._*' -delete
rm -f -- "$ARCHIVE"

echo "Parakeet V3 INT8 installed at $MODEL_DIR"
