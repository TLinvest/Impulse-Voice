# Troubleshooting

Start with the built-in report:

```bash
impulse-voice-daemon --doctor
systemctl --user status impulse-voice.service
journalctl --user -u impulse-voice.service -n 100 --no-pager
```

## “No microphone samples were received”

Confirm PipeWire sees a default source:

```bash
wpctl status
pactl info
pactl list short sources
```

Then list the devices CPAL can open:

```bash
impulse-voice-daemon --list-input-devices
```

To select a device explicitly, create a systemd override:

```bash
systemctl --user edit impulse-voice.service
```

```ini
[Service]
Environment=IMPULSE_VOICE_INPUT_DEVICE=exact CPAL device name
```

Apply it:

```bash
systemctl --user daemon-reload
systemctl --user restart impulse-voice.service
```

Hold the shortcut while speaking. A quick tap can produce an intentionally
empty recording.

## “No speech was detected”

The RMS silence gate rejected the recording. Check that the selected source is
the microphone rather than an output monitor, increase the source level, and
record for at least a quarter of a second.

```bash
wpctl set-volume @DEFAULT_AUDIO_SOURCE@ 0.7
```

## Model missing or incomplete

Run the downloader again:

```bash
./scripts/download-model.sh
```

If an interrupted extraction left an incomplete model directory, move that
directory aside before retrying. The downloader never deletes an existing
model directory automatically.

Expected files:

```text
encoder-model.int8.onnx
decoder_joint-model.int8.onnx
nemo128.onnx
vocab.txt
```

## Shortcut does nothing

Check that Hyprland loaded the managed bindings:

```bash
hyprctl binds | grep -A8 -B2 'Impulse Voice'
```

Check that Quickshell registered the component:

```bash
qs -c ii ipc call impulseVoice toggle
```

If the IPC command works but the shortcut does not, look for another binding
using the same key combination in `~/.config/hypr`.

## Text does not appear in a terminal

Impulse Voice detects common terminal window classes and types text directly
to avoid `Ctrl+Shift` shortcut collisions. Inspect the active class:

```bash
hyprctl activewindow -j | jq -r '.class, .initialClass'
```

Open an issue if your terminal is not detected. Include both class values and
the terminal name, but do not include private transcript text.

## `kded6` conflict dialog after installation

The dialog belongs to Illogical Impulse's conflict checker and can appear when
Quickshell reloads. Select **No**. Impulse Voice does not require `kded6` to be
terminated.

The installer reloads Quickshell only when its installed QML changes, so normal
reinstalls should not repeatedly open the dialog.

## First transcription is slow

Parakeet is loaded lazily. Preload it after login or installation:

```bash
impulse-voice-daemon --warmup
```

Once loaded, the model remains in the daemon until the service restarts.

## Collecting a useful bug report

Include:

- distribution and version;
- Hyprland, Quickshell, and Illogical Impulse revisions;
- output from `impulse-voice-daemon --doctor`;
- relevant service logs;
- output of `impulse-voice-daemon --list-input-devices`;
- the focused application class if insertion failed.

Remove usernames, filesystem paths, device serial numbers, and transcript text
before posting logs publicly.
