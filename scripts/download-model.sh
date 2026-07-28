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
  echo "Parakeet V3 INT8 est déjà installé dans $MODEL_DIR"
  exit 0
fi

if [[ -e "$MODEL_DIR" ]]; then
  echo "Le dossier modèle existe mais il est incomplet : $MODEL_DIR" >&2
  echo "Déplace-le ou supprime-le explicitement avant de relancer le téléchargement." >&2
  exit 1
fi

for command in curl sha256sum tar find mktemp; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "Commande requise introuvable : $command" >&2
    exit 1
  }
done

mkdir -p "$MODEL_ROOT"
echo "Téléchargement de Parakeet V3 INT8 (~478 Mo)…"
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
  echo "Archive invalide : encoder-model.int8.onnx est absent." >&2
  exit 1
fi

source_dir="$(dirname "$encoder_path")"
for file in "${REQUIRED_FILES[@]}"; do
  [[ -f "$source_dir/$file" ]] || {
    echo "Archive invalide : $file est absent." >&2
    exit 1
  }
done

mkdir -p "$MODEL_DIR"
cp -a "$source_dir/." "$MODEL_DIR/"
find "$MODEL_DIR" -type f -name '._*' -delete
rm -f -- "$ARCHIVE"

echo "Parakeet V3 INT8 installé dans $MODEL_DIR"
