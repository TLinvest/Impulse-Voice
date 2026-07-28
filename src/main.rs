use std::{
    env, fs,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    signal,
    sync::Mutex,
    time::{sleep, Duration},
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Override the Unix socket path.
    #[arg(long, env = "IMPULSE_VOICE_SOCKET")]
    socket: Option<PathBuf>,

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
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn default_socket_path() -> Result<PathBuf> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is not defined")?;
    Ok(PathBuf::from(runtime_dir).join("impulse-voice.sock"))
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

async fn handle_client(stream: UnixStream, state: Arc<Mutex<VoiceState>>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    write_message(&mut writer, &message("state", *state.lock().await, None)).await?;

    while let Some(line) = lines.next_line().await? {
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                write_message(
                    &mut writer,
                    &json!({
                        "type": "error",
                        "code": "invalid_request",
                        "message": error.to_string(),
                    }),
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
                    &message("state", *state.lock().await, request.id.as_ref()),
                )
                .await?;
            }
            "start" => {
                *state.lock().await = VoiceState::Listening;
                write_message(
                    &mut writer,
                    &message("state", VoiceState::Listening, request.id.as_ref()),
                )
                .await?;
            }
            "stop" => {
                *state.lock().await = VoiceState::Processing;
                write_message(
                    &mut writer,
                    &message("state", VoiceState::Processing, request.id.as_ref()),
                )
                .await?;

                // Control-plane scaffold: audio capture and Parakeet are connected next.
                sleep(Duration::from_millis(250)).await;
                *state.lock().await = VoiceState::Idle;
                write_message(
                    &mut writer,
                    &json!({
                        "type": "error",
                        "code": "engine_not_connected",
                        "message": "Le moteur audio Parakeet n'est pas encore connecté.",
                        "state": VoiceState::Idle,
                        "id": request.id,
                    }),
                )
                .await?;
            }
            "toggle" => {
                let next = match *state.lock().await {
                    VoiceState::Idle => VoiceState::Listening,
                    VoiceState::Listening | VoiceState::Processing => VoiceState::Idle,
                };
                *state.lock().await = next;
                write_message(&mut writer, &message("state", next, request.id.as_ref())).await?;
            }
            "cancel" => {
                *state.lock().await = VoiceState::Idle;
                write_message(
                    &mut writer,
                    &message("state", VoiceState::Idle, request.id.as_ref()),
                )
                .await?;
            }
            command => {
                write_message(
                    &mut writer,
                    &json!({
                        "type": "error",
                        "code": "unknown_command",
                        "message": format!("Commande inconnue: {command}"),
                        "id": request.id,
                    }),
                )
                .await?;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let socket_path = match args.socket {
        Some(path) => path,
        None => default_socket_path()?,
    };

    if args.print_socket {
        println!("{}", socket_path.display());
        return Ok(());
    }

    prepare_socket(&socket_path)?;
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind {}", socket_path.display()))?;
    let _socket_guard = SocketGuard(socket_path.clone());
    let state = Arc::new(Mutex::new(VoiceState::Idle));

    info!(socket = %socket_path.display(), "Impulse Voice daemon ready");

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(error) = handle_client(stream, state).await {
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
