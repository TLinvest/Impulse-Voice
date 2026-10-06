# Third-party notices

Impulse Voice combines original integration code with open-source libraries and
a separately downloaded speech-recognition model.

## NVIDIA Parakeet TDT 0.6B v3

- Project: <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>
- Copyright: NVIDIA Corporation
- License: Creative Commons Attribution 4.0 International (CC BY 4.0)
- License text: <https://creativecommons.org/licenses/by/4.0/legalcode>

The model is not stored in this repository. The installer downloads an INT8
ONNX conversion published by Handy and verifies a pinned SHA-256 checksum.
Model weights remain subject to NVIDIA's model license.

## Handy

- Project: <https://github.com/cjpais/Handy>
- Copyright: Handy contributors
- License: MIT

Handy inspired the local dictation direction and publishes the Parakeet V3 INT8
archive consumed by the model downloader. Impulse Voice does not use Handy's
name, logo, or product identity.

## transcribe-rs

- Project: <https://github.com/cjpais/transcribe-rs>
- Copyright: transcribe-rs contributors
- License: MIT

`transcribe-rs` provides the Rust API used to load and execute the ONNX
Parakeet model.

## Rust dependencies

The complete resolved dependency graph is recorded in `Cargo.lock`. Each crate
retains its own copyright and license terms. Run a dependency-license scanner
such as `cargo-about` or `cargo-deny` when preparing redistributed binary
packages.
