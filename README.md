# Impulse Voice

Local, private speech-to-text for CachyOS, Hyprland, Quickshell, and Illogical
Impulse.

Impulse Voice is designed as a lightweight alternative to cloud dictation
tools: hold a shortcut, speak, release, and insert the transcript into the
currently focused application. Audio stays on the machine.

## Current status

`0.1.0` is the project foundation:

- executable Rust daemon
- newline-delimited JSON protocol over a Unix socket
- Quickshell service and non-focusable capsule
- Quickshell IPC commands and global shortcuts
- systemd user service

The audio capture and Parakeet inference pipeline are intentionally the next
milestone. The current daemon returns `engine_not_connected` after a test
recording cycle.

## Planned speech pipeline

- PipeWire-compatible capture through `cpal`
- mono 16 kHz resampling
- Silero VAD
- Parakeet TDT 0.6B v3 INT8 through `transcribe-rs` and ONNX Runtime
- local dictionary and text normalization
- insertion through `wl-copy` and `wtype`, with clipboard restoration

See [architecture](docs/architecture.md) and [IPC protocol](docs/protocol.md).

## Development

```bash
cargo run
```

In another terminal:

```bash
printf '{"id":1,"command":"start"}\n' | socat - UNIX-CONNECT:"$XDG_RUNTIME_DIR/impulse-voice.sock"
```

Build an optimized binary:

```bash
cargo build --release
install -Dm755 target/release/impulse-voice-daemon \
  "$HOME/.local/bin/impulse-voice-daemon"
```

## Quickshell integration

The repository mirrors the intended Illogical Impulse destinations:

```text
quickshell/services/ImpulseVoiceService.qml
quickshell/modules/ii/impulseVoice/ImpulseVoice.qml
```

After linking or copying these files:

1. Import `qs.modules.ii.impulseVoice` in
   `panelFamilies/IllogicalImpulseFamily.qml`.
2. Add `PanelLoader { component: ImpulseVoice {} }`.
3. Restart Quickshell.

Available shell commands:

```bash
qs -c ii ipc call impulseVoice start
qs -c ii ipc call impulseVoice stop
qs -c ii ipc call impulseVoice toggle
qs -c ii ipc call impulseVoice cancel
```

## Service

```bash
install -Dm644 systemd/impulse-voice.service \
  "$HOME/.config/systemd/user/impulse-voice.service"
systemctl --user daemon-reload
systemctl --user enable --now impulse-voice.service
```

## Credits and license

The product direction is inspired by
[Handy](https://github.com/cjpais/Handy). Handy is MIT licensed; its name, logo,
and brand assets are not used by this project.

Impulse Voice is licensed under the [MIT License](LICENSE).
