"""Private pipe worker: little-endian f32 PCM in, one JSON response out."""
import json
import os
import struct
import sys

# Keep native/runtime diagnostics off the protocol pipe.
protocol = os.fdopen(os.dup(sys.stdout.fileno()), "w", buffering=1)
os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
os.environ["HF_HUB_OFFLINE"] = "1"
os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"


def reply(value):
    protocol.write(json.dumps(value) + "\n")


def read_exact(size):
    data = bytearray()
    while len(data) < size:
        chunk = sys.stdin.buffer.read(size - len(data))
        if not chunk:
            raise EOFError("incomplete audio request")
        data.extend(chunk)
    return data


def main():
    import moondream as md
    import numpy as np

    with md.photon("moondream/parakeet-redux", model_path=sys.argv[1],
                   device="cpu", cpu_threads=8) as speech:
        reply({"ready": True})
        while header := sys.stdin.buffer.read(4):
            if len(header) != 4:
                raise EOFError("incomplete request header")
            size = struct.unpack("<I", header)[0]
            if size == 0 or size > 16000 * 300 * 4 or size % 4:
                raise ValueError("invalid audio request size")
            audio = np.frombuffer(read_exact(size), dtype="<f4")
            try:
                result = speech.transcribe(audio=audio, sample_rate=16000)
                reply({"text": result["text"]})
            except Exception as error:
                reply({"error": str(error)})


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        reply({"error": str(error)})
        raise
