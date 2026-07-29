use std::{
    env, fs,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use serde_json::Value;

const PASTE_DELAY: Duration = Duration::from_millis(80);
const RESTORE_DELAY: Duration = Duration::from_millis(350);

#[derive(Clone, Copy, Debug)]
enum InsertionMode {
    Standard,
    Terminal,
}

pub fn insert_text(text: &str) -> Result<()> {
    let previous = read_text_clipboard();
    write_text_clipboard(text)?;
    thread::sleep(PASTE_DELAY);
    let mode = detect_insertion_mode();
    tracing::info!(?mode, "inserting transcript");
    insert_with_keyboard(text, mode)?;
    thread::sleep(RESTORE_DELAY);

    match previous {
        Some(content) => write_text_clipboard(&content)?,
        None => {
            let _ = Command::new("wl-copy").arg("--clear").status();
        }
    }
    Ok(())
}

fn read_text_clipboard() -> Option<String> {
    let output = session_command("wl-paste")
        .args(["--no-newline", "--type", "text"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn write_text_clipboard(text: &str) -> Result<()> {
    // wl-copy forks a clipboard owner. Closing its output descriptors avoids
    // waiting forever on pipes inherited by that background process.
    let status = session_command("wl-copy")
        .args(["--", text])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("wl-copy was not found")?;
    if !status.success() {
        bail!("wl-copy failed");
    }
    Ok(())
}

fn detect_insertion_mode() -> InsertionMode {
    let active_window = session_command("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| serde_json::from_slice::<Value>(&output.stdout).ok());

    let is_terminal = active_window.as_ref().is_some_and(|window| {
        ["class", "initialClass"]
            .iter()
            .filter_map(|key| window.get(key).and_then(Value::as_str))
            .any(is_terminal_class)
    });

    if is_terminal {
        InsertionMode::Terminal
    } else {
        InsertionMode::Standard
    }
}

fn is_terminal_class(class: &str) -> bool {
    let class = class.to_ascii_lowercase();
    [
        "alacritty",
        "blackbox",
        "com.mitchellh.ghostty",
        "com.raggesilver.blackbox",
        "foot",
        "footclient",
        "ghostty",
        "gnome-terminal",
        "gnome-terminal-server",
        "io.elementary.terminal",
        "kitty",
        "konsole",
        "lxterminal",
        "org.gnome.console",
        "org.gnome.terminal",
        "org.kde.konsole",
        "org.wezfurlong.wezterm",
        "st",
        "terminator",
        "tilix",
        "urxvt",
        "wezterm",
        "xfce4-terminal",
        "xterm",
    ]
    .contains(&class.as_str())
}

fn insert_with_keyboard(text: &str, mode: InsertionMode) -> Result<()> {
    if command_exists("wtype") {
        let mut command = session_command("wtype");
        match mode {
            // Direct typing avoids synthetic Ctrl+Shift modifiers leaking into
            // global Hyprland shortcuts while a terminal TUI has focus.
            InsertionMode::Terminal => {
                command.args(["--", text]);
            }
            InsertionMode::Standard => {
                command.args(["-M", "ctrl", "-k", "v", "-m", "ctrl"]);
            }
        }
        let output = command.output().context("wtype failed")?;
        if output.status.success() {
            return Ok(());
        }
    }

    if command_exists("ydotool") {
        let keys: &[&str] = match mode {
            InsertionMode::Standard => &["key", "29:1", "47:1", "47:0", "29:0"],
            // Shift+Insert is the conventional terminal paste fallback and
            // avoids the Ctrl+Shift combination bound to the task manager.
            InsertionMode::Terminal => &["key", "42:1", "110:1", "110:0", "42:0"],
        };
        let output = Command::new("ydotool")
            .args(keys)
            .output()
            .context("ydotool failed")?;
        if output.status.success() {
            return Ok(());
        }
    }

    bail!("no working text-insertion tool is available (wtype or ydotool)")
}

fn session_command(program: &str) -> Command {
    let mut command = Command::new(program);
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);

    if env::var_os("WAYLAND_DISPLAY").is_none() {
        if let Some(display) = runtime_dir.as_deref().and_then(find_wayland_display) {
            command.env("WAYLAND_DISPLAY", display);
        }
    }

    if env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        if let Some(signature) = runtime_dir.as_deref().and_then(find_hyprland_signature) {
            command.env("HYPRLAND_INSTANCE_SIGNATURE", signature);
        }
    }

    command
}

fn find_wayland_display(runtime_dir: &Path) -> Option<String> {
    fs::read_dir(runtime_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|file_type| file_type.is_socket())
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| is_wayland_display_name(name))
        .max()
}

fn is_wayland_display_name(name: &str) -> bool {
    name.strip_prefix("wayland-")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

fn find_hyprland_signature(runtime_dir: &Path) -> Option<String> {
    fs::read_dir(runtime_dir.join("hypr"))
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(".socket.sock").exists())
        .max_by_key(|entry| {
            entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
        })
        .and_then(|entry| entry.file_name().into_string().ok())
}

pub fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\" >/dev/null 2>&1", "sh", command])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{find_hyprland_signature, is_terminal_class, is_wayland_display_name};

    fn test_runtime_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("impulse-voice-insertion-{}-{nonce}", process::id()))
    }

    #[test]
    fn detects_common_terminal_classes_case_insensitively() {
        assert!(is_terminal_class("kitty"));
        assert!(is_terminal_class("org.kde.konsole"));
        assert!(is_terminal_class("Alacritty"));
        assert!(is_terminal_class("com.mitchellh.ghostty"));
    }

    #[test]
    fn does_not_treat_regular_apps_as_terminals() {
        assert!(!is_terminal_class("firefox"));
        assert!(!is_terminal_class("code"));
        assert!(!is_terminal_class("org.libreoffice.LibreOffice"));
    }

    #[test]
    fn discovers_session_paths_without_imported_environment() {
        let runtime_dir = test_runtime_dir();
        let signature = "test_hyprland_signature";
        let hypr_dir = runtime_dir.join("hypr").join(signature);
        fs::create_dir_all(&hypr_dir).unwrap();
        fs::write(hypr_dir.join(".socket.sock"), []).unwrap();

        assert!(is_wayland_display_name("wayland-7"));
        assert!(!is_wayland_display_name("wayland-7.lock"));
        assert_eq!(
            find_hyprland_signature(&runtime_dir).as_deref(),
            Some(signature)
        );

        fs::remove_dir_all(runtime_dir).unwrap();
    }
}
