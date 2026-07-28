use std::{
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};

const PASTE_DELAY: Duration = Duration::from_millis(80);
const RESTORE_DELAY: Duration = Duration::from_millis(350);

pub fn insert_text(text: &str) -> Result<()> {
    let previous = read_text_clipboard();
    write_text_clipboard(text)?;
    thread::sleep(PASTE_DELAY);
    send_paste_shortcut()?;
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

fn send_paste_shortcut() -> Result<()> {
    if command_exists("wtype") {
        let output = Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v"])
            .output()
            .context("échec de wtype")?;
        if output.status.success() {
            return Ok(());
        }
    }

    if command_exists("ydotool") {
        let output = Command::new("ydotool")
            .args(["key", "29:1", "47:1", "47:0", "29:0"])
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
