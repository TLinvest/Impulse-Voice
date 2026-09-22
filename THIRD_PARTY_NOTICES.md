# Third-party notices

Impulse Voice combines original integration code with open-source libraries and
a separately downloaded speech-recognition model.

## NVIDIA Parakeet TDT 0.6B v3

- Project: <https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3>
- Copyright: NVIDIA Corporation
- License: Creative Commons Attribution 4.0 International (CC BY 4.0)
- License text: <https://creativecommons.org/licenses/by/4.0/legalcode>

The model is not stored in this repository. Redux is a compressed derivative
published by Moondream under CC BY 4.0.

## Moondream Parakeet Redux and Photon

- Model: <https://huggingface.co/moondream/parakeet-redux>
- Model license: CC BY 4.0
- Runtime: <https://moondream.ai/photon>

The installer downloads a pinned Redux revision and verifies the weights with
SHA-256. Photon is installed through `moondream==2.4.0` with CPU PyTorch in an
isolated environment. Runtime packages and native kernel bundles retain their
respective upstream licenses; the repository's MIT license does not replace them.

## Handy

- Project: <https://github.com/cjpais/Handy>
- License: MIT

Handy inspired the original local dictation direction. The Redux backend no
longer uses Handy's ONNX archive or transcribe-rs.

## Rust dependencies

The complete resolved dependency graph is recorded in `Cargo.lock`. Each crate
retains its own copyright and license terms. Run a dependency-license scanner
such as `cargo-about` or `cargo-deny` when preparing redistributed binary
packages.
