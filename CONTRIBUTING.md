# Contributing to Impulse Voice

Thank you for helping make private Linux dictation better.

## Before opening a change

- Search existing issues and pull requests.
- Keep the tested platform in mind: Hyprland, Quickshell, Illogical Impulse,
  PipeWire, and Arch-family distributions.
- Open an issue before large protocol, model, or UI architecture changes.
- Never add telemetry, cloud transcription, or background microphone capture
  without an explicit design discussion.

## Development setup

Install the packages listed in the README, clone the repository, and run:

```bash
cargo build
cargo test
```

The daemon can be tested without installing the Quickshell component:

```bash
cargo run -- --doctor
cargo run -- --list-input-devices
cargo run -- --no-paste
```

Use a 16 kHz mono WAV fixture for repeatable inference tests:

```bash
cargo run -- --transcribe-wav /path/to/test.wav
```

Do not commit recordings, model weights, generated ONNX files, or transcripts.

## Required checks

Run these before opening a pull request:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
bash -n scripts/*.sh
shellcheck scripts/*.sh
```

If `shellcheck` is unavailable locally, CI will still run it.

## Project conventions

### Rust

- Prefer small modules with explicit ownership boundaries.
- Keep audio callbacks non-blocking and allocation-light.
- Keep model inference outside the async runtime.
- Return actionable errors; do not silently fall back across privacy
  boundaries.
- Add unit tests for pure detection, parsing, normalization, and resampling
  logic.

### QML

- Follow Illogical Impulse's existing component and naming patterns.
- Use theme values from `Appearance`; do not hard-code a second visual system.
- Overlay windows must remain non-focusable.
- Keep the Unix-socket service independent from the visual capsule.

### Installer

- Installation and uninstallation must be idempotent.
- Modify only marked or precisely anchored sections of user configuration.
- Back up a user file before its first structural modification.
- Never delete a model or unrelated user configuration automatically.
- Avoid unnecessary Quickshell reloads.

## Commits and pull requests

Use concise imperative commits when practical:

```text
feat: add terminal class detection
fix: keep the microphone stream alive
docs: explain the local privacy boundary
```

A pull request should explain:

- the user-visible problem;
- the chosen solution;
- how it was tested;
- any privacy, compatibility, memory, or latency impact;
- screenshots for QML changes.

Keep unrelated refactors out of focused fixes.

## Reporting security and privacy issues

Do not open a public issue for a vulnerability that could expose microphone
audio, transcripts, local files, or command execution. Follow
[SECURITY.md](SECURITY.md) instead.
