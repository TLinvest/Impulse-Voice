use std::{
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use serde_json::Value;

const PASTE_DELAY: Duration = Duration::from_millis(80);
const RESTORE_DELAY: Duration = Duration::from_millis(350);

#[derive(Clone, Copy)]
enum PasteShortcut {
    Standard,
    Terminal,
}

pub fn insert_text(text: &str) -> Result<()> {
    let previous = read_text_clipboard();
    write_text_clipboard(text)?;
    thread::sleep(PASTE_DELAY);
    send_paste_shortcut(detect_paste_shortcut())?;
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
    let output = Command::new("wl-paste")
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
    let status = Command::new("wl-copy")
        .args(["--", text])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("wl-copy est introuvable")?;
    if !status.success() {
        bail!("wl-copy a échoué");
    }
    Ok(())
}

fn detect_paste_shortcut() -> PasteShortcut {
    let active_window = Command::new("hyprctl")
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
        PasteShortcut::Terminal
    } else {
        PasteShortcut::Standard
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

fn send_paste_shortcut(shortcut: PasteShortcut) -> Result<()> {
    if command_exists("wtype") {
        let mut command = Command::new("wtype");
        command.args(["-M", "ctrl"]);
        if matches!(shortcut, PasteShortcut::Terminal) {
            command.args(["-M", "shift"]);
        }
        command.args(["-k", "v"]);
        if matches!(shortcut, PasteShortcut::Terminal) {
            command.args(["-m", "shift"]);
        }
        let output = command
            .args(["-m", "ctrl"])
            .output()
            .context("échec de wtype")?;
        if output.status.success() {
            return Ok(());
        }
    }

    if command_exists("ydotool") {
        let keys: &[&str] = match shortcut {
            PasteShortcut::Standard => &["key", "29:1", "47:1", "47:0", "29:0"],
            PasteShortcut::Terminal => &["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"],
        };
        let output = Command::new("ydotool")
            .args(keys)
            .output()
            .context("échec de ydotool")?;
        if output.status.success() {
            return Ok(());
        }
    }

    bail!("aucun outil d'insertion fonctionnel (wtype ou ydotool)")
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
    use super::is_terminal_class;

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
}
