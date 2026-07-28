# Architecture

## Target pipeline

```text
Hyprland shortcut
       |
       v
Quickshell capsule <---- NDJSON / Unix socket ----> Rust daemon
                                                    |
                                      PipeWire/cpal capture
                                                    |
                                      16 kHz mono resampling
                                                    |
                                           Silero VAD
                                                    |
                                   Parakeet TDT 0.6B v3 INT8
                                                    |
                                  dictionary and text normalization
                                                    |
                                      wl-copy + wtype/ydotool
```

## Responsibilities

The Quickshell layer owns presentation, shortcuts, state feedback, and settings.
It must never request keyboard focus while recording, otherwise the transcript
could be pasted into the overlay instead of the previously focused application.

The Rust daemon owns audio capture, VAD, model lifetime, inference, transcript
history, and text insertion. Keeping Parakeet loaded in a user service avoids a
model load on every dictation.

## Planned backend crates

- `cpal` for PipeWire-compatible audio capture
- `rubato` for resampling
- `vad-rs` for Silero voice activity detection
- `transcribe-rs` with its ONNX feature for Parakeet V3 INT8

The current `0.1.0` repository contains the control plane and UI integration.
Audio capture and inference are the next implementation milestone.

## Inspiration

The workflow and separation of concerns are inspired by
[Handy](https://github.com/cjpais/Handy), which is MIT licensed. Impulse Voice
uses its own name and visual identity. No Handy brand assets are used.

