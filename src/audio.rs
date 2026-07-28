use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SampleFormat, SizedSample, Stream, SupportedStreamConfig,
};
use rubato::{FftFixedIn, Resampler};
use serde::Serialize;

pub const TARGET_SAMPLE_RATE: u32 = 16_000;
const MAX_RECORDING_DURATION: Duration = Duration::from_secs(5 * 60);
const RESAMPLER_CHUNK_SIZE: usize = 1024;

#[derive(Debug, Clone, Serialize)]
pub struct InputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Default)]
struct CaptureBuffer {
    samples: Vec<f32>,
    overflowed: bool,
}

struct ActiveRecording {
    _stream: Stream,
    buffer: Arc<Mutex<CaptureBuffer>>,
    sample_rate: u32,
    device_name: String,
    started_at: Instant,
}

pub struct RecordedAudio {
    pub samples: Vec<f32>,
    pub duration: Duration,
    pub device_name: String,
}

#[derive(Default)]
pub struct AudioRecorder {
    active: Option<ActiveRecording>,
    preferred_device: Option<String>,
}

impl AudioRecorder {
    pub fn new(preferred_device: Option<String>) -> Self {
        Self {
            active: None,
            preferred_device,
        }
    }

    pub fn start(&mut self) -> Result<String> {
        if self.active.is_some() {
            bail!("a recording is already in progress");
        }

        let host = cpal::default_host();
        let device = select_input_device(&host, self.preferred_device.as_deref())?;
        let device_name = device.name().unwrap_or_else(|_| "Microphone".to_string());
        let config = preferred_input_config(&device)?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let max_samples = sample_rate as usize * MAX_RECORDING_DURATION.as_secs() as usize;
        let buffer = Arc::new(Mutex::new(CaptureBuffer {
            samples: Vec::with_capacity(sample_rate as usize * 30),
            overflowed: false,
        }));

        let stream = match config.sample_format() {
            SampleFormat::I8 => {
                build_stream::<i8>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::I32 => {
                build_stream::<i32>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::I64 => {
                build_stream::<i64>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::U8 => {
                build_stream::<u8>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::U32 => {
                build_stream::<u32>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::U64 => {
                build_stream::<u64>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, channels, max_samples, &buffer)
            }
            SampleFormat::F64 => {
                build_stream::<f64>(&device, &config, channels, max_samples, &buffer)
            }
            format => bail!("format audio non pris en charge: {format:?}"),
        }?;

        stream.play().context("failed to start the microphone")?;
        self.active = Some(ActiveRecording {
            _stream: stream,
            buffer,
            sample_rate,
            device_name: device_name.clone(),
            started_at: Instant::now(),
        });

        Ok(device_name)
    }

    pub fn stop(&mut self) -> Result<RecordedAudio> {
        let active = self.active.take().context("no recording is in progress")?;
        let duration = active.started_at.elapsed();

        // Dropping the stream stops the callback before we take its buffer.
        let ActiveRecording {
            _stream,
            buffer,
            sample_rate,
            device_name,
            ..
        } = active;
        drop(_stream);

        let mut capture = buffer
            .lock()
            .map_err(|_| anyhow::anyhow!("the audio buffer lock is poisoned"))?;
        if capture.overflowed {
            bail!("the recording exceeds the five-minute limit");
        }
        let raw = std::mem::take(&mut capture.samples);
        drop(capture);

        if raw.is_empty() {
            bail!("no microphone samples were received");
        }

        let samples = resample_to_16khz(&raw, sample_rate)?;
        Ok(RecordedAudio {
            samples,
            duration,
            device_name,
        })
    }

    pub fn cancel(&mut self) {
        self.active.take();
    }
}

fn select_input_device(host: &cpal::Host, preferred_name: Option<&str>) -> Result<Device> {
    if let Some(name) = preferred_name {
        let devices = host
            .input_devices()
            .context("failed to enumerate input devices")?;
        for device in devices {
            if device.name().ok().as_deref() == Some(name) {
                return Ok(device);
            }
        }
        bail!("configured input device not found: {name}");
    }

    host.default_input_device()
        .context("no default input device is available")
}

fn preferred_input_config(device: &Device) -> Result<SupportedStreamConfig> {
    device
        .default_input_config()
        .context("failed to read the input-device configuration")
}

fn build_stream<T>(
    device: &Device,
    config: &SupportedStreamConfig,
    channels: usize,
    max_samples: usize,
    buffer: &Arc<Mutex<CaptureBuffer>>,
) -> Result<Stream>
where
    T: Sample + SizedSample + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let shared = Arc::clone(buffer);
    let stream_config = config.clone().into();
    device
        .build_input_stream(
            &stream_config,
            move |data: &[T], _| {
                let Ok(mut capture) = shared.lock() else {
                    return;
                };
                if capture.overflowed {
                    return;
                }

                let frames = data.len() / channels;
                if capture.samples.len() + frames > max_samples {
                    capture.overflowed = true;
                    return;
                }

                if channels == 1 {
                    capture
                        .samples
                        .extend(data.iter().map(|&sample| sample.to_sample::<f32>()));
                } else {
                    capture.samples.reserve(frames);
                    for frame in data.chunks_exact(channels) {
                        let mono = frame
                            .iter()
                            .map(|&sample| sample.to_sample::<f32>())
                            .sum::<f32>()
                            / channels as f32;
                        capture.samples.push(mono);
                    }
                }
            },
            |error| tracing::error!(%error, "microphone stream error"),
            None,
        )
        .context("failed to create the microphone stream")
}

fn resample_to_16khz(input: &[f32], input_rate: u32) -> Result<Vec<f32>> {
    if input_rate == TARGET_SAMPLE_RATE {
        return Ok(input.to_vec());
    }

    let mut resampler = FftFixedIn::<f32>::new(
        input_rate as usize,
        TARGET_SAMPLE_RATE as usize,
        RESAMPLER_CHUNK_SIZE,
        1,
        1,
    )
    .context("failed to initialize the audio resampler")?;

    let expected_len =
        ((input.len() as u64 * TARGET_SAMPLE_RATE as u64) / input_rate as u64) as usize;
    let output_delay = resampler.output_delay();
    let mut output = Vec::with_capacity(expected_len + output_delay + RESAMPLER_CHUNK_SIZE);

    for chunk in input.chunks(RESAMPLER_CHUNK_SIZE) {
        let mut padded = chunk.to_vec();
        padded.resize(RESAMPLER_CHUNK_SIZE, 0.0);
        let block = resampler
            .process(&[padded.as_slice()], None)
            .context("audio resampling failed")?;
        output.extend_from_slice(&block[0]);
    }

    // Push enough silence through the FFT resampler to expose its delayed tail.
    for _ in 0..4 {
        if output.len() >= expected_len + output_delay {
            break;
        }
        let block = resampler
            .process_partial::<Vec<f32>>(None, None)
            .context("failed to flush the audio resampler")?;
        output.extend_from_slice(&block[0]);
    }

    let start = output_delay.min(output.len());
    let end = (start + expected_len).min(output.len());
    let mut trimmed = output[start..end].to_vec();
    trimmed.resize(expected_len, 0.0);
    Ok(trimmed)
}

pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());
    let devices = host
        .input_devices()
        .context("failed to enumerate input devices")?
        .filter_map(|device| device.name().ok())
        .map(|name| InputDeviceInfo {
            is_default: default_name.as_deref() == Some(name.as_str()),
            name,
        })
        .collect();
    Ok(devices)
}

pub fn probe_default_input() -> Result<(String, SupportedStreamConfig)> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("no default input device is available")?;
    let name = device.name().unwrap_or_else(|_| "Microphone".to_string());
    let config = preferred_input_config(&device)?;
    Ok((name, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_at_target_rate() {
        let input = vec![-0.5, 0.0, 0.5];
        assert_eq!(
            resample_to_16khz(&input, TARGET_SAMPLE_RATE).unwrap(),
            input
        );
    }

    #[test]
    fn resampling_has_expected_length() {
        let input = vec![0.0; 48_000];
        let output = resample_to_16khz(&input, 48_000).unwrap();
        assert_eq!(output.len(), 16_000);
    }
}
