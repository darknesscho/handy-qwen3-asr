#!/usr/bin/env bash
# python_sidecar/setup.sh — Set up Python environment for Qwen3-ASR sidecar
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VENV_DIR="${SCRIPT_DIR}/venv"

echo "=== Setting up Python environment for Qwen3-ASR sidecar ==="

# Ensure uv is available
if ! command -v uv &>/dev/null; then
    echo "[0] Installing uv..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    source "$HOME/.local/bin/env"
fi

# Create venv with Python 3.12 (PyTorch requires 3.12 or older)
if [ ! -d "${VENV_DIR}" ]; then
    echo "[1/4] Creating Python 3.12 virtual environment..."
    uv venv --python 3.12 "${VENV_DIR}"
else
    echo "[1/4] Virtual environment already exists."
fi

echo "[2/4] Installing PyTorch with CUDA..."
uv pip install --python "${VENV_DIR}/bin/python" \
    torch torchvision torchaudio \
    --index-url https://download.pytorch.org/whl/cu124

echo "[3/4] Installing qwen-asr..."
uv pip install --python "${VENV_DIR}/bin/python" -U qwen-asr

echo "[4/4] Installing modelscope (for model download)..."
uv pip install --python "${VENV_DIR}/bin/python" modelscope

echo ""
echo "=== Done ==="
echo ""
echo "To download the model from ModelScope:"
echo "  uv run --python ${VENV_DIR}/bin/python modelscope download --model Qwen/Qwen3-ASR-0.6B"
echo ""
echo "To test the sidecar:"
echo "  ${VENV_DIR}/bin/python ${SCRIPT_DIR}/transcriber.py"
