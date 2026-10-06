# Security policy

## Supported versions

Impulse Voice is currently pre-1.0. Security fixes are applied to the latest
commit on `main`. Older commits and locally modified installations are not
maintained as separate release lines.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting feature when it becomes
available on the public repository.

If private reporting is not enabled yet, contact the repository owner through
their GitHub profile and request a private channel. Do not include exploit
details, recordings, transcripts, tokens, or private paths in a public issue.

A useful report includes:

- affected commit or release;
- impact and realistic attack scenario;
- reproduction steps;
- whether user interaction is required;
- suggested mitigation, if known.

## Security boundaries

Impulse Voice:

- reads microphone audio only during an explicit recording;
- stores audio in memory rather than on disk;
- communicates over a user-runtime Unix socket;
- performs inference locally;
- has no TCP listener, web server, analytics, or cloud API;
- writes only to documented user configuration and install paths.

The installer downloads a pinned model archive over HTTPS and verifies its
SHA-256 checksum. The repository does not redistribute model weights.

## Out of scope

- vulnerabilities in unmodified upstream dependencies;
- malicious model files supplied through a custom model path;
- local attackers already able to control the user's session, clipboard,
  Wayland compositor, or configuration files;
- unsupported desktop environments or installation scripts modified by third
  parties.

Responsible reports will be acknowledged in release notes unless the reporter
prefers to remain anonymous.
