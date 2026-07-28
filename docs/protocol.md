# IPC protocol

Impulse Voice uses newline-delimited JSON over:

```text
$XDG_RUNTIME_DIR/impulse-voice.sock
```

## Commands

```json
{"id":1,"command":"ping"}
{"id":2,"command":"status"}
{"id":3,"command":"start"}
{"id":4,"command":"stop"}
{"id":5,"command":"toggle"}
{"id":6,"command":"cancel"}
```

## Events

```json
{"type":"state","state":"idle"}
{"type":"state","state":"listening","id":3}
{"type":"state","state":"processing","id":4}
{"type":"transcript","text":"Bonjour depuis Impulse Voice.","language":"fr"}
{"type":"error","code":"audio_device_unavailable","message":"Microphone indisponible."}
```

States are `idle`, `listening`, and `processing`. The protocol is deliberately
small so Quickshell, a CLI client, or another desktop shell can use the daemon.

