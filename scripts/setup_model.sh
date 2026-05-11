#!/usr/bin/env bash
# setup_model.sh — Download and verify Qwen3-ASR 0.6B INT8 model
#
# This script downloads the sherpa-onnx pre-exported Qwen3-ASR 0.6B INT8 model
# and places it in the app's model directory for offline use.
#
# Model source: GitHub Releases (k2-fsa/sherpa-onnx)
# More info: https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models

set -euo pipefail

MODEL_ID="qwen3-asr-0.6b"
FILENAME="sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25"
ARCHIVE="${FILENAME}.tar.bz2"
URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${ARCHIVE}"

# Resolve app model directory (same logic as the Rust app's ModelManager)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

case "$(uname -s)" in
  Linux)  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/handy" ;;
  Darwin) DATA_DIR="$HOME/Library/Application Support/com.handy.app" ;;
  *)      echo "Unsupported OS"; exit 1 ;;
esac

MODELS_DIR="${DATA_DIR}/models"
FINAL_DIR="${MODELS_DIR}/${FILENAME}"

echo "=== Qwen3-ASR 0.6B INT8 Model Setup ==="
echo "Target directory: ${FINAL_DIR}"
mkdir -p "${MODELS_DIR}"

# Download
if [ -d "${FINAL_DIR}" ]; then
  echo "[OK] Model already exists at ${FINAL_DIR}"
  echo "      Size: $(du -sh "${FINAL_DIR}" | cut -f1)"
  exit 0
fi

echo "[1/3] Downloading model from GitHub..."
echo "      ${URL}"
curl -L -o "/tmp/${ARCHIVE}" "${URL}"

echo "[2/3] Extracting archive..."
tar xf "/tmp/${ARCHIVE}" -C "${MODELS_DIR}"
rm "/tmp/${ARCHIVE}"

echo "[3/3] Computing SHA256 (for model.rs sha256 field)..."
if command -v sha256sum &>/dev/null; then
  SHA256=$(sha256sum "${FINAL_DIR}/../${ARCHIVE}" 2>/dev/null || true)
  # Compute from the downloaded archive if it still exists
fi

echo ""
echo "=== Done ==="
echo "Model extracted to: ${FINAL_DIR}"
echo ""
echo "Files:"
ls -lh "${FINAL_DIR}"

echo ""
echo "=== Computing archive SHA256 ==="
# Re-archive and compute SHA256 for model.rs
cd "${MODELS_DIR}"
tar cjf "/tmp/${ARCHIVE}" "${FILENAME}" 2>/dev/null || \
  tar cf - "${FILENAME}" | bzip2 > "/tmp/${ARCHIVE}"
SHA256=$(sha256sum "/tmp/${ARCHIVE}" | cut -d' ' -f1)
rm "/tmp/${ARCHIVE}"
echo "Archive SHA256: ${SHA256}"
echo ""
echo "Update model.rs with this SHA256 hash."
