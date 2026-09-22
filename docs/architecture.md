# Architecture

Impulse Voice is split into a desktop-facing Quickshell component and a
long-running Rust daemon. The boundary is intentionally narrow: newline-
delimited JSON over a user-only Unix socket.

```text
Hyprland global shortcut
          │
          ▼
Illogical Impulse waveform ◄─── NDJSON / Unix socket ────► Rust daemon
                                                               │
                                                        CPAL microphone
                                                               │
                                                mono downmix + Rubato 16 kHz
                                                               │
                                                   energy-based silence trim
                                                               │
                                              Parakeet Redux / Photon CPU
                                                               │
                                                   transcript normalization
                                                               │
                                            wtype / clipboard-aware insertion
```

## Dictation lifecycle

The microphone opens only after a `start` command. A `stop` command drops the
CPAL stream, takes ownership of the sample buffer, resamples it, trims quiet
edges, and sends the resulting PCM to Parakeet. This prevents a persistent
microphone indicator while Impulse Voice is idle.

Parakeet is loaded lazily during the first transcription and remains warm for
10 minutes after its last use. It is then unloaded automatically to release its
memory; a later transcription reloads it on demand. Inference and text insertion
run on blocking worker threads so the Tokio socket remains responsive.

The recorder limits a single capture to five minutes. Impulse Voice is designed
for push-to-talk dictation rather than meetings or continuous transcription.

## Illogical Impulse integration

The installer makes three scoped desktop changes:

1. copies `ImpulseVoiceService.qml` into the Illogical Impulse service tree;
2. copies the capsule module into `modules/ii/impulseVoice`;
3. adds one import and one `PanelLoader` to
   `IllogicalImpulseFamily.qml`.

It also owns a clearly marked block in
`~/.config/hypr/custom/keybinds.conf`. Re-running the installer updates only
that managed block. The original panel-family file is backed up before its
first modification.

The waveform is a non-focusable overlay with a zero exclusion zone. It appears
directly below the desktop bar and contains no text, timer, or microphone icon.
Its eleven center-aligned bars react to the daemon's normalized RMS meter while
listening, then use state-specific animations while processing, completing, or
reporting an error. The application that owned keyboard focus before dictation
keeps it throughout the recording and processing states.

## Audio path

CPAL opens the selected ALSA input exposed by PipeWire. The callback:

1. converts the native sample format to `f32`;
2. averages multichannel frames into mono;
3. updates a smoothed, normalized RMS meter for the Quickshell waveform;
4. appends frames to a bounded in-memory buffer.

On stop, Rubato converts the native sample rate to 16 kHz. A lightweight RMS
gate removes quiet leading and trailing windows while retaining padding around
speech.

## Inference

The model directory must contain:

```text
parakeet-redux/
├── model.safetensors
├── config.json
├── ternary.json
└── tokenizer.json
```

The downloader fetches a pinned Hugging Face revision and verifies the weights'
SHA-256. The Rust daemon starts an embedded Python worker using a dedicated
Photon environment. Little-endian float32 mono PCM is length-prefixed over a
private stdin pipe; results are JSON lines over stdout. Runtime diagnostics go
to stderr. Hugging Face offline mode prevents runtime downloads. The worker
stays warm until idle unloading, and is terminated and reaped on drop or error.
The next dictation starts a fresh worker after an inference failure.

The upstream NVIDIA model is licensed under CC BY 4.0. The Impulse Voice source
code is MIT licensed; the downloaded model remains governed by its own license.

## Text insertion

Insertion is selected from the active Hyprland window class:

- regular applications receive a clipboard paste through `wtype`;
- terminal emulators receive direct text input, avoiding global shortcut
  collisions from synthetic `Ctrl+Shift` modifiers;
- `ydotool` provides a fallback when `wtype` is unavailable.

The previous text clipboard is restored after insertion. Impulse Voice does
not retain transcripts.

## Trust boundaries

- The Unix socket lives under `$XDG_RUNTIME_DIR` and inherits the user's runtime
  directory permissions.
- Audio stays in memory within the daemon and its local Photon child process.
- The daemon does not expose TCP, HTTP, telemetry, or update endpoints.
- The installer downloads the model and runtime dependencies; inference is offline.
- Shell integration is limited to the current user's configuration and
  systemd user manager.

## Current limitations

- Linux, Wayland, Hyprland, and Illogical Impulse are the tested stack.
- The current inference path is CPU-oriented.
- Silence detection uses RMS energy rather than a neural VAD.
- The integration expects the current Illogical Impulse panel-family layout.
- There is no streaming partial transcript; text appears after release.

A future hands-free or streaming mode should replace the RMS gate with a
dedicated VAD and introduce explicit privacy controls before keeping a
microphone stream open.
