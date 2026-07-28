# IPC protocol

Impulse Voice uses newline-delimited JSON (NDJSON) over a Unix socket:

```text
$XDG_RUNTIME_DIR/impulse-voice.sock
```

One JSON object is written per line. A client may keep the connection open and
send multiple commands. The daemon sends an initial state event immediately
after connection.

## Request shape

```json
{
  "id": 42,
  "command": "stop",
  "paste": false
}
```

| Field | Required | Description |
| --- | --- | --- |
| `command` | Yes | One of the commands below |
| `id` | No | Any JSON value echoed in related responses |
| `paste` | No | Per-request insertion override for `stop` or `toggle` |

## Commands

```json
{"id":1,"command":"ping"}
{"id":2,"command":"status"}
{"id":3,"command":"meter"}
{"id":4,"command":"start"}
{"id":5,"command":"stop"}
{"id":6,"command":"stop","paste":false}
{"id":7,"command":"toggle"}
{"id":8,"command":"cancel"}
```

| Command | Effect |
| --- | --- |
| `ping` | Check liveness |
| `status` | Return state and model information |
| `meter` | Return the current normalized microphone level |
| `start` | Open the input stream and begin buffering |
| `stop` | Stop capture, transcribe, and optionally insert |
| `toggle` | Start while idle; stop while listening |
| `cancel` | Drop the current recording without transcription |

## Events

State transition:

```json
{"type":"state","state":"listening","device":"default","id":3}
```

Audio meter sample:

```json
{"type":"meter","state":"listening","level":0.62,"id":4}
```

`level` is an RMS-derived value normalized to the `0.0`–`1.0` range. Clients
may poll it while the daemon is listening; the bundled Quickshell component
uses a 40 ms interval and smooths the resulting bar animation.

Successful transcript:

```json
{
  "type": "transcript",
  "state": "idle",
  "text": "Hello from Impulse Voice.",
  "pasted": true,
  "paste_error": null,
  "duration_ms": 1840,
  "samples": 29440,
  "device": "default",
  "id": 4
}
```

Error:

```json
{
  "type": "error",
  "code": "audio_start_failed",
  "message": "No default input device is available",
  "state": "idle",
  "id": 3
}
```

States are:

- `idle`
- `listening`
- `processing`

## Connection example

Keep one connection open so `start` and `stop` share the same client session:

```bash
ncat -U "$XDG_RUNTIME_DIR/impulse-voice.sock"
```

Then enter:

```json
{"id":1,"command":"start"}
{"id":2,"command":"stop","paste":false}
```

The protocol deliberately contains no Illogical Impulse-specific types. A CLI
client or another desktop shell can reuse the daemon without importing the QML
component.

## Compatibility

The protocol is currently unversioned and pre-1.0. Additive response fields
may be introduced without a version bump. Clients should ignore unknown fields
and rely on `type`, `state`, and `code` rather than matching complete objects.
