use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{bail, Context, Result};
use transcribe_rs::onnx::{
    parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity},
    Quantization,
};

const REQUIRED_MODEL_FILES: &[&str] = &[
    "encoder-model.int8.onnx",
    "decoder_joint-model.int8.onnx",
    "nemo128.onnx",
    "vocab.txt",
];
const SAMPLE_RATE: usize = 16_000;
const MIN_AUDIO_SAMPLES: usize = SAMPLE_RATE / 4;
const SILENCE_RMS_THRESHOLD: f32 = 0.004;
const TRIM_WINDOW_SAMPLES: usize = 160;
const TRIM_PADDING_SAMPLES: usize = SAMPLE_RATE / 4;

pub struct Transcriber {
    model_path: PathBuf,
    model: Mutex<Option<ParakeetModel>>,
}

impl Transcriber {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            model: Mutex::new(None),
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
        let mut model_guard = self
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("le moteur Parakeet est indisponible"))?;
        if model_guard.is_none() {
            tracing::info!(path = %self.model_path.display(), "loading Parakeet V3 INT8");
            let model = ParakeetModel::load(&self.model_path, &Quantization::Int8)
                .context("échec du chargement de Parakeet V3 INT8")?;
            *model_guard = Some(model);
            tracing::info!("Parakeet V3 INT8 loaded");
        }
        Ok(())
    }

    pub fn transcribe(&self, samples: Vec<f32>) -> Result<String> {
        let audio = trim_silence(&samples).context("aucune parole détectée")?;
        if audio.len() < MIN_AUDIO_SAMPLES {
            bail!("enregistrement trop court");
        }

        self.warmup()?;
        let mut model_guard = self
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("le moteur Parakeet est indisponible"))?;
        let params = ParakeetParams {
            timestamp_granularity: Some(TimestampGranularity::Segment),
            ..Default::default()
        };
        let result = model_guard
            .as_mut()
            .expect("model initialized above")
            .transcribe_with(audio, &params)
            .context("échec de la transcription Parakeet")?;
        let text = normalize_transcript(&result.text);
        if text.is_empty() {
            bail!("Parakeet n'a produit aucun texte");
        }
        Ok(text)
    }

    pub fn transcribe_wav(&self, path: &Path) -> Result<String> {
        let samples = transcribe_rs::audio::read_wav_samples(path)
            .with_context(|| format!("lecture WAV impossible: {}", path.display()))?;
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
            "modèle Parakeet incomplet dans {} (fichiers manquants: {})",
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
