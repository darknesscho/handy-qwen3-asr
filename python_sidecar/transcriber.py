#!/usr/bin/env python3
"""
Qwen3-ASR transcriber sidecar for Handy.

Long-lived process that loads Qwen3-ASR once and transcribes audio
sent via stdin/stdout JSON-line protocol.

Protocol (one JSON object per line):

  → {"audio": "<base64_pcm_f32>", "sample_rate": 16000}
  ← {"text": "<transcription>", "language": "<detected_lang>"}

Errors:

  ← {"error": "<message>"}

Startup: writes {"ready": true} to stdout when model is loaded.
"""

import base64
import json
import os
import sys
import struct
import numpy as np
import torch

from qwen_asr import Qwen3ASRModel


def main():
    # Load model from ModelScope cache
    model_path = os.path.expanduser(
        "~/.cache/modelscope/hub/models/Qwen/Qwen3-ASR-0___6B"
    )
    if not os.path.exists(model_path):
        # Fallback to HuggingFace ID if not cached locally
        model_path = "Qwen/Qwen3-ASR-0.6B"

    model = Qwen3ASRModel.from_pretrained(
        model_path,
        dtype=torch.bfloat16,
        device_map="cuda:0",
        max_inference_batch_size=1,
        max_new_tokens=256,
    )

    # Signal ready
    sys.stdout.write(json.dumps({"ready": True}) + "\n")
    sys.stdout.flush()

    # Process incoming audio
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            msg = json.loads(line)
        except json.JSONDecodeError as e:
            sys.stdout.write(
                json.dumps({"error": f"invalid JSON: {e}"}) + "\n"
            )
            sys.stdout.flush()
            continue

        if "audio" not in msg:
            sys.stdout.write(json.dumps({"error": "missing 'audio' field"}) + "\n")
            sys.stdout.flush()
            continue

        try:
            audio_bytes = base64.b64decode(msg["audio"])
            sample_rate = msg.get("sample_rate", 16000)

            # Convert raw bytes to float32 numpy array
            audio_f32 = np.frombuffer(audio_bytes, dtype=np.float32)

            # Transcribe (audio is tuple of (numpy_array, sample_rate))
            results = model.transcribe(
                audio=(audio_f32, sample_rate),
                language=msg.get("language"),
            )

            result = results[0]
            response = {
                "text": result.text,
                "language": result.language if hasattr(result, "language") else None,
            }
        except Exception as e:
            response = {"error": str(e)}

        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
