# Changelog

All notable changes to Impulse Voice will be documented here.

The project follows [Semantic Versioning](https://semver.org/) after the first
public release. While the project is pre-1.0, minor versions may include
integration changes.

## [Unreleased]

## [0.2.0] - 2026-07-28

### Added

- Public project documentation and GitHub community files
- Native Illogical Impulse Quickshell capsule
- Push-to-talk and toggle-mode global shortcuts
- PipeWire/ALSA microphone capture through CPAL
- Mono conversion and 16 kHz resampling
- Local Parakeet TDT 0.6B v3 INT8 inference
- Context-aware text insertion and clipboard restoration
- User-level systemd service and NDJSON Unix-socket protocol
- Idempotent model downloader, installer, and uninstaller
- Diagnostic, warmup, input-device listing, and WAV transcription commands

### Fixed

- Quickshell reloads on unchanged reinstalls
- Hold-to-talk shortcut lifecycle
- Terminal paste modifier collisions

[Unreleased]: https://github.com/TLinvest/Impulse-Voice/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/TLinvest/Impulse-Voice/releases/tag/v0.2.0
