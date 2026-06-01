use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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

/// Metadata about an input device, returned by [`list_input_devices`].
pub struct DeviceInfo {
    pub index: usize,
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
            let name = d.name().unwrap_or_else(|_| "<unknown>".into());
            let config = d
                .default_input_config()
                .map(|c| format!("{} ch, {} Hz", c.channels(), c.sample_rate().0))
                .unwrap_or_else(|_| "n/a".into());
            DeviceInfo {
                index: i,
                name,
                config,
                is_default: i == 0,
            }
        })
        .collect())
}

pub struct Recorder {
    device: cpal::Device,
    native_channels: u16,
    native_sample_rate: u32,
}

impl Recorder {
    /// Initialise recorder with the given `device_filter` (from config), or prompt
    /// interactively if `None`. Uses the device's native format.
    ///
    /// Returns `(Self, Option<String>)` — the second value is `Some(filter)` if a
    /// new device was selected interactively and should be persisted.
    pub fn new(device_filter: Option<&str>) -> Result<(Self, Option<String>)> {
        let host = cpal::default_host();
        let devices: Vec<cpal::Device> = host
            .input_devices()
            .context("No audio input devices found.")?
            .collect();

        if devices.is_empty() {
            anyhow::bail!("No input devices available.");
        }

        // Device selection:
        // 1. Config device_filter → use it (no output)
        // 2. None                 → list devices to console, prompt, persist result
        let (device, newly_selected) = if let Some(filter) = device_filter {
            (pick_device_by_filter(&devices, filter), None)
        } else {
            // Print device list to console only (not to log)
            eprintln!("┌─ Available input devices ────────────────────────────");
            for (i, d) in devices.iter().enumerate() {
                let name = d.name().unwrap_or_else(|_| "<unknown>".into());
                let cfg = d
                    .default_input_config()
                    .map(|c| format!("{} ch, {} Hz", c.channels(), c.sample_rate().0));
                let marker = if i == 0 { " (default)" } else { "" };
                eprintln!(
                    "│ [{n}] {name} — {cfg}{marker}",
                    n = i + 1,
                    cfg = cfg.as_deref().unwrap_or("n/a"),
                );
            }
            eprintln!("└──────────────────────────────────────────────────────");

            eprint!("\n🎙  Select device [1]: ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            let input = input.trim().to_string();
            if input.is_empty() {
                tracing::info!("🎙  Using default device [1]");
                (devices.into_iter().next().unwrap(), Some("1".to_string()))
            } else {
                let chosen = input.clone();
                (pick_device_by_filter(&devices, &input), Some(chosen))
            }
        };

        let default_config = device.default_input_config()?;
        let native_channels = default_config.channels();
        let native_sample_rate = default_config.sample_rate().0;

        tracing::info!(
            "🎙  Using: {name} | {ch} ch, {rate} Hz → mono 16 kHz",
            name = device.name().unwrap_or_else(|_| "<unknown>".into()),
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
            sample_rate: cpal::SampleRate(self.native_sample_rate),
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

/// Select a device by substring match on name, or by numeric index.
fn pick_device_by_filter(devices: &[cpal::Device], filter: &str) -> cpal::Device {
    let filter_lower = filter.to_lowercase();

    // Try substring match
    if let Some(d) = devices.iter().find(|d| {
        d.name()
            .map(|n| n.to_lowercase().contains(&filter_lower))
            .unwrap_or(false)
    }) {
        tracing::info!("🎙  Matched device by name: {filter}");
        return d.clone();
    }

    // Try numeric index (1-based from user / config)
    if let Ok(idx) = filter.parse::<usize>() {
        if idx >= 1 {
            if let Some(d) = devices.get(idx - 1) {
                tracing::info!("🎙  Selected device [{idx}]");
                return d.clone();
            }
        }
    }

    // Fallback
    tracing::warn!("🎙  '{filter}' — no match, using default [0]");
    devices.first().cloned().unwrap()
}
