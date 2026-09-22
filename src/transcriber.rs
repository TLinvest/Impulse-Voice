use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const REQUIRED_MODEL_FILES: &[&str] = &[
    "model.safetensors",
    "config.json",
    "tokenizer.json",
    "ternary.json",
];

struct PhotonWorker {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Drop for PhotonWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl PhotonWorker {
    fn start(path: &Path) -> Result<Self> {
        let python = std::env::var_os("IMPULSE_VOICE_PYTHON")
            .map(PathBuf::from)
            .unwrap_or(crate::paths::data_home()?.join("impulse-voice/photon-venv/bin/python"));
        let mut child = Command::new(&python)
            .args(["-u", "-c", include_str!("../runtime/photon_worker.py")])
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to start Photon using {}; run scripts/install-runtime.sh",
                    python.display()
                )
            })?;
        let input = child.stdin.take().context("missing Photon stdin")?;
        let output = BufReader::new(child.stdout.take().context("missing Photon stdout")?);
        let mut worker = Self {
            child,
            input,
            output,
        };
        let ready = worker.response()?;
        if ready.get("ready").and_then(|v| v.as_bool()) != Some(true) {
            bail!("Photon did not signal readiness");
        }
        Ok(worker)
    }

    fn response(&mut self) -> Result<serde_json::Value> {
        let mut line = String::new();
        if self.output.by_ref().take(1_048_576).read_line(&mut line)? == 0 {
            bail!("Photon worker exited unexpectedly; see service logs");
        }
        let result: serde_json::Value =
            serde_json::from_str(&line).context("invalid Photon response")?;
        if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
            bail!("Photon: {error}");
        }
        Ok(result)
    }

    fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        if samples.len() > SAMPLE_RATE * 300 {
            bail!("recording exceeds five minutes");
        }
        let mut bytes = Vec::with_capacity(4 + samples.len() * 4);
        bytes.extend_from_slice(&((samples.len() * 4) as u32).to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.input.write_all(&bytes)?;
        self.input.flush()?;
        let result = self.response()?;
        Ok(result
            .get("text")
            .and_then(|v| v.as_str())
            .context("missing Photon transcript")?
            .to_owned())
    }
}

const SAMPLE_RATE: usize = 16_000;
const MIN_AUDIO_SAMPLES: usize = SAMPLE_RATE / 4;
const SILENCE_RMS_THRESHOLD: f32 = 0.004;
const TRIM_WINDOW_SAMPLES: usize = 160;
const TRIM_PADDING_SAMPLES: usize = SAMPLE_RATE / 4;

pub struct Transcriber {
    model_path: PathBuf,
    state: Mutex<TranscriberState>,
}

struct TranscriberState {
    model: Option<PhotonWorker>,
    last_used: Option<Instant>,
}

impl Transcriber {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            state: Mutex::new(TranscriberState {
                model: None,
                last_used: None,
            }),
        }
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub fn model_ready(&self) -> bool {
        model_files_present(&self.model_path)
    }

    pub fn warmup(&self) -> Result<()> {
        validate_model_directory(&self.model_path)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("the Parakeet engine is unavailable"))?;
        self.load_model_if_needed(&mut state)?;
        state.last_used = Some(Instant::now());
        Ok(())
    }

    fn load_model_if_needed(&self, state: &mut TranscriberState) -> Result<()> {
        if state.model.is_none() {
            tracing::info!(path = %self.model_path.display(), "loading Parakeet Redux via Photon CPU");
            state.model = Some(PhotonWorker::start(&self.model_path)?);
            tracing::info!("Parakeet Redux loaded");
        }
        Ok(())
    }

    pub fn transcribe(&self, samples: Vec<f32>) -> Result<String> {
        let audio = trim_silence(&samples).context("no speech was detected")?;
        if audio.len() < MIN_AUDIO_SAMPLES {
            bail!("the recording is too short");
        }

        validate_model_directory(&self.model_path)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("the Parakeet engine is unavailable"))?;
        self.load_model_if_needed(&mut state)?;
        let result = state
            .model
            .as_mut()
            .expect("model initialized above")
            .transcribe(audio);
        state.last_used = Some(Instant::now());
        if result.is_err() {
            // Retry with a fresh worker next time after a crash or inference error.
            state.model.take();
        }
        let text = normalize_transcript(&result.context("Parakeet Redux transcription failed")?);
        if text.is_empty() {
            bail!("Parakeet produced no text");
        }
        Ok(text)
    }

    pub fn unload_if_idle(&self, idle_timeout: Duration) -> Result<bool> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("the Parakeet engine is unavailable"))?;
        let Some(last_used) = state.last_used else {
            return Ok(false);
        };
        if last_used.elapsed() < idle_timeout {
            return Ok(false);
        }

        let unloaded = state.model.take().is_some();
        state.last_used = None;
        Ok(unloaded)
    }

    pub fn transcribe_wav(&self, path: &Path) -> Result<String> {
        let mut reader = hound::WavReader::open(path)
            .with_context(|| format!("failed to read WAV file: {}", path.display()))?;
        let spec = reader.spec();
        if spec.channels != 1 || spec.sample_rate != SAMPLE_RATE as u32 {
            bail!("WAV input must be mono 16 kHz");
        }
        let samples = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
            hound::SampleFormat::Int => {
                let scale = 2_f32.powi(spec.bits_per_sample as i32 - 1);
                reader
                    .samples::<i32>()
                    .map(|v| v.map(|v| v as f32 / scale))
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        self.transcribe(samples)
    }
}

pub fn model_files_present(path: &Path) -> bool {
    REQUIRED_MODEL_FILES
        .iter()
        .all(|file| path.join(file).is_file())
}

pub fn validate_model_directory(path: &Path) -> Result<()> {
    let missing: Vec<_> = REQUIRED_MODEL_FILES
        .iter()
        .filter(|file| !path.join(file).is_file())
        .copied()
        .collect();
    if !missing.is_empty() {
        bail!(
            "incomplete Parakeet model at {} (missing files: {})",
            path.display(),
            missing.join(", ")
        );
    }
    Ok(())
}

fn trim_silence(samples: &[f32]) -> Option<&[f32]> {
    let active_windows: Vec<usize> = samples
        .chunks(TRIM_WINDOW_SAMPLES)
        .enumerate()
        .filter_map(|(index, window)| {
            let power = window.iter().map(|sample| sample * sample).sum::<f32>()
                / window.len().max(1) as f32;
            (power.sqrt() >= SILENCE_RMS_THRESHOLD).then_some(index)
        })
        .collect();

    let first = *active_windows.first()? * TRIM_WINDOW_SAMPLES;
    let last = ((*active_windows.last()? + 1) * TRIM_WINDOW_SAMPLES).min(samples.len());
    let start = first.saturating_sub(TRIM_PADDING_SAMPLES);
    let end = (last + TRIM_PADDING_SAMPLES).min(samples.len());
    Some(&samples[start..end])
}

fn normalize_transcript(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run with IMPULSE_VOICE_TEST_WAV set to a spoken mono 16 kHz WAV.
    #[test]
    #[ignore = "requires installed Photon, Redux weights and a speech fixture"]
    fn photon_reuses_worker_and_reloads_after_idle() {
        let wav =
            PathBuf::from(std::env::var_os("IMPULSE_VOICE_TEST_WAV").expect("set speech fixture"));
        let transcriber = Transcriber::new(crate::paths::default_model_path().unwrap());
        let first = transcriber.transcribe_wav(&wav).unwrap();
        assert!(!first.is_empty());
        let pid = transcriber
            .state
            .lock()
            .unwrap()
            .model
            .as_ref()
            .unwrap()
            .child
            .id();
        assert_eq!(transcriber.transcribe_wav(&wav).unwrap(), first);
        assert_eq!(
            transcriber
                .state
                .lock()
                .unwrap()
                .model
                .as_ref()
                .unwrap()
                .child
                .id(),
            pid
        );
        assert!(!transcriber
            .unload_if_idle(Duration::from_secs(600))
            .unwrap());
        assert!(transcriber.unload_if_idle(Duration::ZERO).unwrap());
        assert!(transcriber.state.lock().unwrap().model.is_none());
        assert_eq!(transcriber.transcribe_wav(&wav).unwrap(), first);
    }

    #[test]
    fn silence_is_rejected() {
        assert!(trim_silence(&vec![0.0; SAMPLE_RATE]).is_none());
    }

    #[test]
    fn speech_is_kept_with_padding() {
        let mut audio = vec![0.0; SAMPLE_RATE];
        audio[8_000..8_800].fill(0.2);
        let trimmed = trim_silence(&audio).unwrap();
        assert!(trimmed.len() >= 800 + TRIM_PADDING_SAMPLES * 2);
        assert!(trimmed.len() < audio.len());
    }

    #[test]
    fn transcript_whitespace_is_normalized() {
        assert_eq!(
            normalize_transcript(" bonjour  le\nmonde "),
            "bonjour le monde"
        );
    }
}
