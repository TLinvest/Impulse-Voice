mod audio;
mod insertion;
mod paths;
mod transcriber;

use std::{
    fs,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{bail, Context, Result};
use audio::AudioRecorder;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    signal,
    sync::Mutex as AsyncMutex,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use transcriber::Transcriber;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Override the Unix socket path.
    #[arg(long, env = "IMPULSE_VOICE_SOCKET")]
    socket: Option<PathBuf>,

    /// Override the Parakeet V3 INT8 model directory.
    #[arg(long, env = "IMPULSE_VOICE_MODEL")]
    model: Option<PathBuf>,

    /// Select an input device by its exact CPAL name.
    #[arg(long, env = "IMPULSE_VOICE_INPUT_DEVICE")]
    input_device: Option<String>,

    /// Produce transcripts without inserting them into the focused application.
    #[arg(long, env = "IMPULSE_VOICE_NO_PASTE")]
    no_paste: bool,

    /// Print a JSON diagnostic and exit.
    #[arg(long)]
    doctor: bool,

    /// List detected input devices and exit.
    #[arg(long)]
    list_input_devices: bool,

    /// Transcribe a 16 kHz mono PCM WAV file, print the result, and exit.
    #[arg(long, value_name = "PATH")]
    transcribe_wav: Option<PathBuf>,

    /// Load Parakeet into memory to validate the ONNX model, then exit.
    #[arg(long)]
    warmup: bool,

    /// Print the resolved socket path and exit.
    #[arg(long)]
    print_socket: bool,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "snake_case")]
enum VoiceState {
    #[default]
    Idle,
    Listening,
    Processing,
}

#[derive(Debug, Deserialize)]
struct Request {
    #[serde(default)]
    id: Option<Value>,
    command: String,
    #[serde(default)]
    paste: Option<bool>,
}

struct App {
    state: AsyncMutex<VoiceState>,
    recorder: Mutex<AudioRecorder>,
    transcriber: Arc<Transcriber>,
    paste_enabled: bool,
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn prepare_socket(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    if !path.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {}", path.display()))?;
    if !metadata.file_type().is_socket() {
        bail!("refusing to replace non-socket path at {}", path.display());
    }

    fs::remove_file(path).with_context(|| format!("failed to remove stale {}", path.display()))
}

fn message(kind: &str, state: VoiceState, id: Option<&Value>) -> Value {
    let mut value = json!({
        "type": kind,
        "state": state,
    });
    if let Some(id) = id {
        value["id"] = id.clone();
    }
    value
}

async fn write_message(writer: &mut tokio::net::unix::OwnedWriteHalf, value: &Value) -> Result<()> {
    writer
        .write_all(serde_json::to_string(value)?.as_bytes())
        .await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

async fn send_error(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    code: &str,
    error: impl std::fmt::Display,
    id: Option<Value>,
    state: VoiceState,
) -> Result<()> {
    write_message(
        writer,
        &json!({
            "type": "error",
            "code": code,
            "message": error.to_string(),
            "state": state,
            "id": id,
        }),
    )
    .await
}

async fn start_recording(
    app: &App,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    id: Option<Value>,
) -> Result<()> {
    let mut state = app.state.lock().await;
    if !matches!(*state, VoiceState::Idle) {
        return send_error(writer, "busy", "Impulse Voice est occupé", id, *state).await;
    }

    let start_result = {
        let mut recorder = app.recorder.lock().expect("recorder mutex");
        recorder.start()
    };
    let device_name = match start_result {
        Ok(device) => device,
        Err(error) => {
            return send_error(writer, "audio_start_failed", error, id, VoiceState::Idle).await;
        }
    };
    *state = VoiceState::Listening;
    write_message(
        writer,
        &json!({
            "type": "state",
            "state": VoiceState::Listening,
            "device": device_name,
            "id": id,
        }),
    )
    .await
}

async fn stop_and_transcribe(
    app: Arc<App>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    id: Option<Value>,
    paste_override: Option<bool>,
) -> Result<()> {
    {
        let mut state = app.state.lock().await;
        if !matches!(*state, VoiceState::Listening) {
            return send_error(
                writer,
                "not_recording",
                "aucun enregistrement en cours",
                id,
                *state,
            )
            .await;
        }
        *state = VoiceState::Processing;
    }
    write_message(
        writer,
        &message("state", VoiceState::Processing, id.as_ref()),
    )
    .await?;

    let stop_result = {
        let mut recorder = app.recorder.lock().expect("recorder mutex");
        recorder.stop()
    };
    let recording = match stop_result {
        Ok(recording) => recording,
        Err(error) => {
            *app.state.lock().await = VoiceState::Idle;
            return send_error(writer, "audio_stop_failed", error, id, VoiceState::Idle).await;
        }
    };

    let sample_count = recording.samples.len();
    let duration_ms = recording.duration.as_millis() as u64;
    let device_name = recording.device_name;
    let transcriber = Arc::clone(&app.transcriber);
    let transcript = match tokio::task::spawn_blocking(move || {
        transcriber.transcribe(recording.samples)
    })
    .await
    {
        Ok(Ok(text)) => text,
        Ok(Err(error)) => {
            *app.state.lock().await = VoiceState::Idle;
            return send_error(writer, "transcription_failed", error, id, VoiceState::Idle).await;
        }
        Err(error) => {
            *app.state.lock().await = VoiceState::Idle;
            return send_error(
                writer,
                "transcription_task_failed",
                error,
                id,
                VoiceState::Idle,
            )
            .await;
        }
    };

    let should_paste = paste_override.unwrap_or(app.paste_enabled);
    let paste_error = if should_paste {
        let text = transcript.clone();
        tokio::task::spawn_blocking(move || insertion::insert_text(&text))
            .await
            .ok()
            .and_then(Result::err)
            .map(|error| error.to_string())
    } else {
        None
    };

    *app.state.lock().await = VoiceState::Idle;
    write_message(
        writer,
        &json!({
            "type": "transcript",
            "state": VoiceState::Idle,
            "text": transcript,
            "pasted": should_paste && paste_error.is_none(),
            "paste_error": paste_error,
            "duration_ms": duration_ms,
            "samples": sample_count,
            "device": device_name,
            "id": id,
        }),
    )
    .await
}

async fn handle_client(stream: UnixStream, app: Arc<App>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    write_message(
        &mut writer,
        &json!({
            "type": "state",
            "state": *app.state.lock().await,
            "model_ready": app.transcriber.model_ready(),
            "model_path": paths::display_path(app.transcriber.model_path()),
        }),
    )
    .await?;

    while let Some(line) = lines.next_line().await? {
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                send_error(
                    &mut writer,
                    "invalid_request",
                    error,
                    None,
                    *app.state.lock().await,
                )
                .await?;
                continue;
            }
        };

        match request.command.as_str() {
            "ping" => {
                write_message(&mut writer, &json!({ "type": "pong", "id": request.id })).await?;
            }
            "status" => {
                write_message(
                    &mut writer,
                    &json!({
                        "type": "state",
                        "state": *app.state.lock().await,
                        "model_ready": app.transcriber.model_ready(),
                        "model_path": paths::display_path(app.transcriber.model_path()),
                        "id": request.id,
                    }),
                )
                .await?;
            }
            "start" => start_recording(&app, &mut writer, request.id).await?,
            "stop" => {
                stop_and_transcribe(Arc::clone(&app), &mut writer, request.id, request.paste)
                    .await?
            }
            "toggle" => {
                let current_state = {
                    let state = app.state.lock().await;
                    *state
                };
                match current_state {
                    VoiceState::Idle => start_recording(&app, &mut writer, request.id).await?,
                    VoiceState::Listening => {
                        stop_and_transcribe(
                            Arc::clone(&app),
                            &mut writer,
                            request.id,
                            request.paste,
                        )
                        .await?
                    }
                    VoiceState::Processing => {
                        send_error(
                            &mut writer,
                            "busy",
                            "transcription en cours",
                            request.id,
                            VoiceState::Processing,
                        )
                        .await?
                    }
                }
            }
            "cancel" => {
                {
                    let mut recorder = app.recorder.lock().expect("recorder mutex");
                    recorder.cancel();
                }
                *app.state.lock().await = VoiceState::Idle;
                write_message(
                    &mut writer,
                    &message("state", VoiceState::Idle, request.id.as_ref()),
                )
                .await?;
            }
            command => {
                send_error(
                    &mut writer,
                    "unknown_command",
                    format!("commande inconnue: {command}"),
                    request.id,
                    *app.state.lock().await,
                )
                .await?;
            }
        }
    }

    Ok(())
}

fn print_doctor(model_path: &Path) -> Result<()> {
    let audio = audio::probe_default_input()
        .map(|(name, config)| {
            json!({
                "ok": true,
                "device": name,
                "sample_rate": config.sample_rate().0,
                "channels": config.channels(),
                "sample_format": format!("{:?}", config.sample_format()),
            })
        })
        .unwrap_or_else(|error| json!({ "ok": false, "error": error.to_string() }));
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "audio": audio,
            "model": {
                "ok": transcriber::model_files_present(model_path),
                "path": paths::display_path(model_path),
            },
            "tools": {
                "wl_copy": insertion::command_exists("wl-copy"),
                "wl_paste": insertion::command_exists("wl-paste"),
                "wtype": insertion::command_exists("wtype"),
                "ydotool": insertion::command_exists("ydotool"),
            }
        }))?
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("impulse_voice_daemon=info,transcribe_rs=warn,ort=warn")
        }))
        .init();

    let args = Args::parse();
    let socket_path = args.socket.unwrap_or(paths::default_socket_path()?);
    let model_path = args.model.unwrap_or(paths::default_model_path()?);

    if args.print_socket {
        println!("{}", socket_path.display());
        return Ok(());
    }
    if args.list_input_devices {
        println!(
            "{}",
            serde_json::to_string_pretty(&audio::list_input_devices()?)?
        );
        return Ok(());
    }
    if args.doctor {
        return print_doctor(&model_path);
    }
    if args.warmup {
        Transcriber::new(model_path).warmup()?;
        println!("Parakeet V3 INT8 chargé avec succès.");
        return Ok(());
    }
    if let Some(wav_path) = args.transcribe_wav {
        let text = Transcriber::new(model_path).transcribe_wav(&wav_path)?;
        println!("{text}");
        return Ok(());
    }

    prepare_socket(&socket_path)?;
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind {}", socket_path.display()))?;
    let _socket_guard = SocketGuard(socket_path.clone());
    let app = Arc::new(App {
        state: AsyncMutex::new(VoiceState::Idle),
        recorder: Mutex::new(AudioRecorder::new(args.input_device)),
        transcriber: Arc::new(Transcriber::new(model_path.clone())),
        paste_enabled: !args.no_paste,
    });

    info!(
        socket = %socket_path.display(),
        model = %model_path.display(),
        model_ready = app.transcriber.model_ready(),
        paste_enabled = app.paste_enabled,
        "Impulse Voice daemon ready"
    );

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let app = Arc::clone(&app);
                tokio::spawn(async move {
                    if let Err(error) = handle_client(stream, app).await {
                        warn!(%error, "client disconnected with an error");
                    }
                });
            }
            _ = signal::ctrl_c() => {
                info!("shutdown requested");
                break;
            }
        }
    }

    Ok(())
}
