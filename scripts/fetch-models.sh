#!/usr/bin/env bash
# Fetch the local voice models (KWS + Silero VAD) into src-tauri/resources/models.
# These are gitignored binaries (~14 MB). Run once before `npm run dev` / building.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/resources/models"
KWS_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01.tar.bz2"
VAD_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx"

mkdir -p "$DEST/kws"

if [ ! -f "$DEST/silero_vad.onnx" ]; then
  echo "Downloading silero_vad.onnx…"
  curl -sL -o "$DEST/silero_vad.onnx" "$VAD_URL"
fi

if [ ! -f "$DEST/kws/encoder.onnx" ]; then
  echo "Downloading KWS model…"
  tmp="$(mktemp -d)"
  curl -sL -o "$tmp/kws.tar.bz2" "$KWS_URL"
  tar xjf "$tmp/kws.tar.bz2" -C "$tmp"
  src="$tmp/sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01"
  cp "$src/encoder-epoch-12-avg-2-chunk-16-left-64.onnx" "$DEST/kws/encoder.onnx"
  cp "$src/decoder-epoch-12-avg-2-chunk-16-left-64.onnx" "$DEST/kws/decoder.onnx"
  cp "$src/joiner-epoch-12-avg-2-chunk-16-left-64.onnx"  "$DEST/kws/joiner.onnx"
  cp "$src/tokens.txt" "$DEST/kws/tokens.txt"
  cp "$src/bpe.model"  "$DEST/kws/bpe.model"
  rm -rf "$tmp"
fi

# Export bpe.vocab (piece<TAB>score) from bpe.model — the app tokenizes the user's
# configurable wake/end phrases at runtime from this vocab (pure Rust, no deps).
if [ ! -f "$DEST/kws/bpe.vocab" ]; then
  echo "Generating bpe.vocab (needs python3 + sentencepiece)…"
  venv="$(mktemp -d)/venv"
  python3 -m venv "$venv"
  "$venv/bin/pip" install --quiet sentencepiece
  "$venv/bin/python" - "$DEST/kws/bpe.model" "$DEST/kws/bpe.vocab" <<'PY'
import sys, sentencepiece as spm
sp = spm.SentencePieceProcessor(); sp.load(sys.argv[1])
with open(sys.argv[2], "w", encoding="utf-8") as f:
    for i in range(sp.get_piece_size()):
        f.write(f"{sp.id_to_piece(i)}\t{sp.get_score(i)}\n")
print("wrote", sys.argv[2])
PY
fi

echo "Models ready in $DEST"
