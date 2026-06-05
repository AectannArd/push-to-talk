use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::DeviceId;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Holds the ongoing recording: an active audio stream and a shared sample buffer.
pub struct Recording {
    stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<i16>>>,
    /// Set to `true` once the first non-zero sample is captured (for diagnostics).
    has_signal: Arc<AtomicBool>,
}

impl Recording {
    /// Stop recording and return the captured mono 16-bit PCM at 16 kHz.
    pub fn stop(self) -> Vec<i16> {
        tracing::info!("🛑 Recording::stop() called, dropping stream...");
        drop(self.stream); // stops the stream
        tracing::info!("🛑 Audio stream dropped");

        if !self.has_signal.load(Ordering::Relaxed) {
            tracing::warn!("⚠  All audio samples were zero — is the mic muted?");
        }

        // Clone the samples instead of trying to take ownership of the Arc
        // The audio callback may still hold a reference to the Arc
        let samples = self
            .buffer
            .lock()
            .expect("🛑 Recording::stop: Mutex poisoned")
            .clone();

        tracing::info!("🛑 Recording::stop() returning {} samples", samples.len());
        samples
    }
}

pub struct Recorder {
    device: cpal::Device,
    native_channels: u16,
    native_sample_rate: u32,
}

impl Recorder {
    /// Initialise recorder with the given `device_id` (from config), or prompt
    /// interactively if `None`. Uses the device's native format.
    ///
    /// Returns `(Self, Option<(String, String)>)` — the second value is `Some((id, name))` if a
    /// new device was selected interactively and should be persisted.
    pub fn new(device_id: Option<&str>) -> Result<(Self, Option<(String, String)>)> {
        let host = cpal::default_host();

        // Device selection:
        // 1. Config device_id (system ID) → use it (no output)
        // 2. None                         → use default device (GUI mode, no prompt)
        let (device, newly_selected) = if let Some(id) = device_id {
            match pick_device_by_id(&host, id) {
                Some(d) => (d, None),
                None => {
                    tracing::warn!(
                        "Configured device '{id}' not found, falling back to default device"
                    );
                    // Fall back to default device
                    let device = host
                        .default_input_device()
                        .context("No input device available")?;
                    let id = device.id().map(|id| id.to_string()).unwrap_or_default();
                    let name = device
                        .description()
                        .map(|d| d.name().to_string())
                        .unwrap_or_default();
                    (device, Some((id, name)))
                }
            }
        } else {
            // Use default device without prompting (GUI mode)
            let device = host
                .default_input_device()
                .context("No input device available")?;
            let id = device.id().map(|id| id.to_string()).unwrap_or_default();
            let name = device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_default();
            (device, Some((id, name)))
        };

        let default_config = device.default_input_config()?;
        let native_channels = default_config.channels();
        let native_sample_rate = default_config.sample_rate();

        tracing::info!(
            "🎙  Using: {name} | {ch} ch, {rate} Hz → mono 16 kHz",
            name = device
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_else(|_| "<unknown>".into()),
            ch = native_channels,
            rate = native_sample_rate,
        );

        Ok((
            Self {
                device,
                native_channels,
                native_sample_rate,
            },
            newly_selected,
        ))
    }

    /// Begin capturing audio. Returns a [`Recording`] that can be stopped for the buffer.
    ///
    /// Audio is converted on the fly: multi-channel → mono, native rate → 16 kHz,
    /// f32 → i16. The output buffer always contains 16 kHz mono PCM.
    pub fn start(&self) -> Result<Recording> {
        let buffer = Arc::new(Mutex::new(Vec::<i16>::new()));
        let buf = buffer.clone();

        let channels = self.native_channels as usize;
        let ratio = self.native_sample_rate as f64 / 16_000.0;
        let has_signal = Arc::new(AtomicBool::new(false));
        let sig = has_signal.clone();

        // For diagnostics: log the first few raw f32 frames
        let sample_count = Arc::new(AtomicBool::new(false));
        let logged = sample_count.clone();

        // Fractional position for sample-rate conversion
        let position = Arc::new(Mutex::new(0.0f64));
        let pos = position.clone();

        let stream_config = cpal::StreamConfig {
            channels: self.native_channels,
            sample_rate: self.native_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let err_fn = |err| tracing::error!("⚠  Audio stream error: {err}");

        let stream = self.device.build_input_stream(
            &stream_config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                let mut buf = buf.lock().unwrap();
                let mut pos = pos.lock().unwrap();

                // Log first few raw frames for diagnostics (once)
                if !logged.load(Ordering::Relaxed) {
                    logged.store(true, Ordering::Relaxed);
                    let preview: Vec<String> = data
                        .iter()
                        .take(4 * channels)
                        .map(|v| format!("{v:+.6}"))
                        .collect();
                    tracing::info!(
                        "🔬 Raw audio (first {n} samples): [{s}]",
                        n = 4 * channels,
                        s = preview.join(", "),
                    );
                }

                for frame in data.chunks(channels) {
                    let mono = if channels == 1 {
                        frame[0]
                    } else {
                        frame.iter().sum::<f32>() / channels as f32
                    };

                    if mono.abs() > 0.0001 {
                        sig.store(true, Ordering::Relaxed);
                    }

                    *pos += 1.0;
                    if *pos >= ratio {
                        *pos -= ratio;
                        let clamped = mono.clamp(-1.0, 1.0);
                        buf.push((clamped * i16::MAX as f32) as i16);
                    }
                }
            },
            err_fn,
            None,
        )?;

        stream.play()?;

        Ok(Recording {
            stream,
            buffer,
            has_signal,
        })
    }
}

/// Metadata about an audio input device.
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub config: String,
    pub is_default: bool,
}

/// Enumerate all available audio input devices.
pub fn list_input_devices() -> Result<Vec<DeviceInfo>> {
    let host = cpal::default_host();
    let devices: Vec<cpal::Device> = host
        .input_devices()
        .context("No audio input devices found.")?
        .collect();

    Ok(devices
        .into_iter()
        .enumerate()
        .map(|(i, d)| {
            let id = d
                .id()
                .map(|id| id.to_string())
                .unwrap_or_else(|_| "<unknown>".into());
            let name = d
                .description()
                .map(|d| d.name().to_string())
                .unwrap_or_else(|_| "<unknown>".into());
            let config = d
                .default_input_config()
                .map(|c| format!("{} ch, {} Hz", c.channels(), c.sample_rate()))
                .unwrap_or_else(|_| "n/a".into());
            DeviceInfo {
                id,
                name,
                config,
                is_default: i == 0,
            }
        })
        .collect())
}

/// Select a device by its stable system ID.
fn pick_device_by_id(host: &cpal::Host, id_str: &str) -> Option<cpal::Device> {
    match DeviceId::from_str(id_str) {
        Ok(device_id) => {
            if let Some(d) = host.device_by_id(&device_id) {
                tracing::info!("🎙  Selected device by system ID: {id_str}");
                return Some(d);
            }
            tracing::warn!("Device with system ID '{id_str}' not found");
        }
        Err(_) => {
            tracing::warn!("Invalid system device ID format: {id_str}");
        }
    }
    None
}
