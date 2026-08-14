//! Microphone capture engine.
//!
//! Ported from Handy's `audio_toolkit::audio::recorder`, stripped of the VAD
//! and Tauri coupling. A background worker thread owns the cpal input stream,
//! downmixes to mono f32, resamples to 16 kHz (via [`FrameResampler`]), and
//! emits fixed 30 ms frames to two optional callbacks:
//!
//! - a real-time audio-frame callback (used to feed live streaming), and
//! - a level-meter callback (16 spectrum buckets for the dock waveform).
//!
//! `stop()` returns the full captured 16 kHz mono buffer for batch fallback.

use std::{
    io::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SizedSample,
};

use crate::{visualizer::AudioVisualiser, FrameResampler};

/// Target sample rate for whisper.cpp models.
pub const WHISPER_SAMPLE_RATE: u32 = 16000;

enum Cmd {
    /// Begin capturing. Carries a one-shot acknowledgement sent only after the
    /// first microphone sample chunk is processed.
    Start(mpsc::Sender<()>),
    Stop(mpsc::Sender<Vec<f32>>),
    Shutdown,
}

enum AudioChunk {
    Samples(Vec<f32>),
    EndOfStream,
}

/// Callback invoked with each 16 kHz mono frame while recording. Used to feed a
/// live streaming transcription as audio arrives. Keep it cheap (forward to a
/// channel) so it never stalls capture.
pub type AudioFrameCallback = Arc<dyn Fn(&[f32]) + Send + Sync + 'static>;

pub struct AudioRecorder {
    device: Option<Device>,
    cmd_tx: Option<mpsc::Sender<Cmd>>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
    level_cb: Option<Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>>,
    audio_cb: Option<AudioFrameCallback>,
    /// Which input channel to use. None = average all channels.
    selected_channel: Option<usize>,
    /// Preferred stream config cached per device name to skip slow HAL queries.
    config_cache: Arc<Mutex<Option<(String, cpal::SupportedStreamConfig)>>>,
}

impl AudioRecorder {
    /// Creates an audio recorder without opening a device.
    ///
    /// # Errors
    ///
    /// Reserved for recorder initialization failures.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            device: None,
            cmd_tx: None,
            worker_handle: None,
            level_cb: None,
            audio_cb: None,
            selected_channel: None,
            config_cache: Arc::new(Mutex::new(None)),
        })
    }

    #[must_use]
    pub fn with_level_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        self.level_cb = Some(Arc::new(cb));
        self
    }

    /// Register a callback that receives real-time 16 kHz frames while
    /// recording. Frames arrive in real time, in order, on the recorder's
    /// consumer thread.
    #[must_use]
    pub fn with_audio_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(&[f32]) + Send + Sync + 'static,
    {
        self.audio_cb = Some(Arc::new(cb));
        self
    }

    /// Opens the selected input device and starts its capture worker.
    ///
    /// # Errors
    ///
    /// Returns an error if no input device exists or the device stream cannot
    /// be configured, built, or started.
    pub fn open(&mut self, device: Option<Device>) -> Result<(), Box<dyn std::error::Error>> {
        if self.worker_handle.is_some() {
            if !self.is_capture_worker_dead() {
                return Ok(()); // already open
            }
            log::warn!("Capture worker exited; rebuilding microphone stream");
            let _ = self.close();
        }

        let (sample_tx, sample_rx) = mpsc::channel::<AudioChunk>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (init_tx, init_rx) = mpsc::sync_channel::<Result<(), String>>(1);

        let host = crate::get_cpal_host();
        let device = match device {
            Some(dev) => dev,
            None => host
                .default_input_device()
                .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "No input device found"))?,
        };

        let thread_device = device.clone();
        let level_cb = self.level_cb.clone();
        let audio_cb = self.audio_cb.clone();
        let selected_channel = self.selected_channel;
        let config_cache = Arc::clone(&self.config_cache);

        let worker = std::thread::spawn(move || {
            let stop_flag = Arc::new(AtomicBool::new(false));
            let init_result = Self::create_input_stream(
                &thread_device,
                sample_tx,
                selected_channel,
                Arc::clone(&stop_flag),
                &config_cache,
            );

            match init_result {
                Ok((stream, sample_rate)) => {
                    let _ = init_tx.send(Ok(()));
                    run_consumer(ConsumerArgs {
                        sample_rate,
                        sample_rx,
                        cmd_rx,
                        level_cb,
                        audio_cb,
                        stop_flag,
                    });
                    drop(stream);
                }
                Err(error_message) => {
                    if let Ok(mut cache) = config_cache.lock() {
                        *cache = None;
                    }
                    log::error!("{error_message}");
                    let _ = init_tx.send(Err(error_message));
                }
            }
        });

        match init_rx.recv() {
            Ok(Ok(())) => {
                self.device = Some(device);
                self.cmd_tx = Some(cmd_tx);
                self.worker_handle = Some(worker);
                Ok(())
            }
            Ok(Err(error_message)) => {
                let _ = worker.join();
                let kind = if is_microphone_access_denied(&error_message) {
                    std::io::ErrorKind::PermissionDenied
                } else {
                    std::io::ErrorKind::Other
                };
                Err(Box::new(Error::new(kind, error_message)))
            }
            Err(recv_error) => {
                let _ = worker.join();
                Err(Box::new(Error::other(format!(
                    "Failed to initialize microphone worker: {recv_error}"
                ))))
            }
        }
    }

    fn create_input_stream(
        device: &Device,
        sample_tx: mpsc::Sender<AudioChunk>,
        selected_channel: Option<usize>,
        stop_flag: Arc<AtomicBool>,
        config_cache: &Mutex<Option<(String, cpal::SupportedStreamConfig)>>,
    ) -> Result<(cpal::Stream, u32), String> {
        let device_name = device.name().unwrap_or_default();
        let cached_config = config_cache
            .lock()
            .map_err(|_| "Microphone config cache lock was poisoned".to_owned())?
            .as_ref()
            .filter(|(name, _)| !device_name.is_empty() && *name == device_name)
            .map(|(_, config)| config.clone());
        let config_was_cached = cached_config.is_some();
        let config = match cached_config {
            Some(config) => config,
            None => Self::get_preferred_config(device)
                .map_err(|error| format!("Failed to fetch preferred config: {error}"))?,
        };
        let sample_rate = config.sample_rate().0;
        let channel_count = config.channels();
        let channels = usize::from(channel_count);

        log::info!(
            "Using device: {:?} rate={} channels={} format={:?}",
            device.name(),
            sample_rate,
            channels,
            config.sample_format()
        );

        let stream = match config.sample_format() {
            cpal::SampleFormat::U8 => Self::build_stream::<u8>,
            cpal::SampleFormat::I8 => Self::build_stream::<i8>,
            cpal::SampleFormat::I16 => Self::build_stream::<i16>,
            cpal::SampleFormat::I32 => Self::build_stream::<i32>,
            cpal::SampleFormat::F32 => Self::build_stream::<f32>,
            sample_format => return Err(format!("Unsupported sample format: {sample_format:?}")),
        }(
            device,
            &config,
            sample_tx,
            channels,
            f32::from(channel_count),
            selected_channel,
            stop_flag,
        )
        .map_err(|error| format!("Failed to build input stream: {error}"))?;

        stream
            .play()
            .map_err(|error| format!("Failed to start microphone stream: {error}"))?;

        if !config_was_cached && !device_name.is_empty() {
            *config_cache
                .lock()
                .map_err(|_| "Microphone config cache lock was poisoned".to_owned())? =
                Some((device_name, config));
        }

        Ok((stream, sample_rate))
    }

    /// Queue a recording start and return a one-shot receiver that resolves only
    /// after the first real microphone sample chunk has entered the capture path.
    ///
    /// # Errors
    ///
    /// Returns an error if the recorder is not open or its worker has stopped.
    pub fn start(&self) -> Result<mpsc::Receiver<()>, Box<dyn std::error::Error>> {
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| Error::other("Recorder is not open"))?;
        let (ready_tx, ready_rx) = mpsc::channel();
        tx.send(Cmd::Start(ready_tx))?;
        Ok(ready_rx)
    }

    /// Stops capture and returns all samples recorded in the current session.
    ///
    /// # Errors
    ///
    /// Returns an error if the recorder is not open or its worker has stopped.
    pub fn stop(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let (resp_tx, resp_rx) = mpsc::channel();
        let tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| Error::other("Recorder is not open"))?;
        tx.send(Cmd::Stop(resp_tx))?;
        Ok(resp_rx.recv()?)
    }

    /// True once the capture worker has exited without anyone calling `close`.
    #[must_use]
    pub fn is_capture_worker_dead(&self) -> bool {
        self.worker_handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    /// Shuts down the capture worker and releases its input device.
    ///
    /// # Errors
    ///
    /// This method currently completes cleanup on a best-effort basis and
    /// reserves its error result for future platform-specific failures.
    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(Cmd::Shutdown);
        }
        if let Some(h) = self.worker_handle.take() {
            let _ = h.join();
        }
        self.device = None;
        Ok(())
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        sample_tx: mpsc::Sender<AudioChunk>,
        channels: usize,
        channel_divisor: f32,
        selected_channel: Option<usize>,
        stop_flag: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: Sample + SizedSample + Send + 'static,
        f32: cpal::FromSample<T>,
    {
        let mut output_buffer = Vec::new();
        let mut eos_sent = false;
        let use_channel: Option<usize> = match selected_channel {
            Some(ch) if ch < channels => Some(ch),
            Some(_) | None => None,
        };

        let stream_cb = move |data: &[T], _: &cpal::InputCallbackInfo| {
            if stop_flag.load(Ordering::Relaxed) {
                if !eos_sent {
                    let _ = sample_tx.send(AudioChunk::EndOfStream);
                    eos_sent = true;
                }
                return;
            }
            eos_sent = false;

            output_buffer.clear();

            if channels == 1 {
                output_buffer.extend(data.iter().map(|&sample| sample.to_sample::<f32>()));
            } else {
                let frame_count = data.len().checked_div(channels).unwrap_or_default();
                output_buffer.reserve(frame_count);

                if let Some(ch) = use_channel {
                    for frame in data.chunks_exact(channels) {
                        if let Some(sample) = frame.get(ch) {
                            output_buffer.push((*sample).to_sample::<f32>());
                        }
                    }
                } else {
                    for frame in data.chunks_exact(channels) {
                        let mono_sample = frame
                            .iter()
                            .map(|&sample| sample.to_sample::<f32>())
                            .sum::<f32>()
                            / channel_divisor;
                        output_buffer.push(mono_sample);
                    }
                }
            }

            if sample_tx
                .send(AudioChunk::Samples(output_buffer.clone()))
                .is_err()
            {
                log::error!("Failed to send samples");
            }
        };

        device.build_input_stream(
            &config.clone().into(),
            stream_cb,
            |err| log::error!("Stream error: {err}"),
            None,
        )
    }

    fn get_preferred_config(
        device: &cpal::Device,
    ) -> Result<cpal::SupportedStreamConfig, Box<dyn std::error::Error>> {
        // Use the device's native rate and let the FrameResampler downsample to
        // 16 kHz; forcing a non-native rate breaks some Bluetooth/ALSA devices.
        let default_config = device.default_input_config()?;
        let target_rate = default_config.sample_rate();

        let supported_configs = match device.supported_input_configs() {
            Ok(configs) => configs,
            Err(e) => {
                log::warn!("Could not enumerate input configs ({e}), using device default");
                return Ok(default_config);
            }
        };
        let mut best_config: Option<cpal::SupportedStreamConfigRange> = None;

        for config_range in supported_configs {
            if config_range.min_sample_rate() <= target_rate
                && config_range.max_sample_rate() >= target_rate
            {
                match best_config {
                    None => best_config = Some(config_range),
                    Some(ref current) => {
                        let score = |fmt: cpal::SampleFormat| match fmt {
                            cpal::SampleFormat::F32 => 4,
                            cpal::SampleFormat::I16 => 3,
                            cpal::SampleFormat::I32 => 2,
                            _ => 1,
                        };
                        if score(config_range.sample_format()) > score(current.sample_format()) {
                            best_config = Some(config_range);
                        }
                    }
                }
            }
        }

        if let Some(config) = best_config {
            return Ok(config.with_sample_rate(target_rate));
        }

        log::warn!(
            "No supported config matched device default rate {target_rate:?}, using default config"
        );
        Ok(default_config)
    }
}

#[must_use]
pub fn is_microphone_access_denied(error_message: &str) -> bool {
    let normalized = error_message.to_lowercase();
    normalized.contains("access is denied")
        || normalized.contains("permission denied")
        || normalized.contains("0x80070005")
}

#[must_use]
pub fn is_no_input_device_error(error_message: &str) -> bool {
    let normalized = error_message.to_lowercase();
    normalized.contains("no input device found")
        || (normalized.contains("failed to fetch preferred config")
            && normalized.contains("coreaudio"))
}

struct ConsumerArgs {
    sample_rate: u32,
    sample_rx: mpsc::Receiver<AudioChunk>,
    cmd_rx: mpsc::Receiver<Cmd>,
    level_cb: Option<Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>>,
    audio_cb: Option<AudioFrameCallback>,
    stop_flag: Arc<AtomicBool>,
}

fn handle_frame(samples: &[f32], audio_cb: Option<&AudioFrameCallback>, out_buf: &mut Vec<f32>) {
    out_buf.extend_from_slice(samples);
    if let Some(cb) = audio_cb {
        cb(samples);
    }
}

fn run_consumer(args: ConsumerArgs) {
    const BUCKETS: usize = 16;

    let ConsumerArgs {
        sample_rate: in_sample_rate,
        sample_rx,
        cmd_rx,
        level_cb,
        audio_cb,
        stop_flag,
    } = args;
    let Ok(input_rate) = usize::try_from(in_sample_rate) else {
        log::error!("Input sample rate is unsupported on this platform");
        return;
    };
    let Ok(output_rate) = usize::try_from(WHISPER_SAMPLE_RATE) else {
        log::error!("Whisper sample rate is unsupported on this platform");
        return;
    };
    let Ok(mut frame_resampler) =
        FrameResampler::new(input_rate, output_rate, Duration::from_millis(30))
    else {
        log::error!("Failed to create audio frame resampler");
        return;
    };

    let mut processed_samples = Vec::<f32>::new();
    let mut recording = false;

    let target_window = usize::try_from(in_sample_rate.saturating_add(15) / 30).unwrap_or_default();
    let window_size = [256usize, 512, 1024, 2048]
        .into_iter()
        .min_by_key(|w| w.abs_diff(target_window))
        .unwrap_or(256);
    let mut visualizer = AudioVisualiser::new(in_sample_rate, window_size, BUCKETS, 400.0, 4000.0);

    let mut capture_ready_tx: Option<mpsc::Sender<()>> = None;

    while let Ok(chunk) = sample_rx.recv() {
        let mut pending = Some(chunk);
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Cmd::Start(ready_tx) => {
                    capture_ready_tx = Some(ready_tx);
                    stop_flag.store(false, Ordering::Relaxed);
                    processed_samples.clear();
                    recording = true;
                    visualizer.reset();
                    frame_resampler.reset();
                }
                Cmd::Stop(reply_tx) => {
                    recording = false;
                    capture_ready_tx = None;
                    stop_flag.store(true, Ordering::Relaxed);

                    if let Some(AudioChunk::Samples(raw)) = pending.take() {
                        frame_resampler.push(&raw, &mut |frame: &[f32]| {
                            handle_frame(frame, audio_cb.as_ref(), &mut processed_samples);
                        });
                    }

                    loop {
                        match sample_rx.recv_timeout(Duration::from_secs(2)) {
                            Ok(AudioChunk::Samples(remaining)) => {
                                frame_resampler.push(&remaining, &mut |frame: &[f32]| {
                                    handle_frame(frame, audio_cb.as_ref(), &mut processed_samples);
                                });
                            }
                            Ok(AudioChunk::EndOfStream) => break,
                            Err(_) => {
                                log::warn!("Timed out waiting for EndOfStream from audio callback");
                                break;
                            }
                        }
                    }

                    frame_resampler.finish(&mut |frame: &[f32]| {
                        handle_frame(frame, audio_cb.as_ref(), &mut processed_samples);
                    });

                    let _ = reply_tx.send(std::mem::take(&mut processed_samples));
                    stop_flag.store(false, Ordering::Relaxed);
                }
                Cmd::Shutdown => {
                    stop_flag.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }

        let Some(AudioChunk::Samples(raw)) = pending.take() else {
            continue;
        };

        if recording {
            if let Some(buckets) = visualizer.feed(&raw) {
                if let Some(cb) = &level_cb {
                    cb(buckets);
                }
            }

            frame_resampler.push(&raw, &mut |frame: &[f32]| {
                handle_frame(frame, audio_cb.as_ref(), &mut processed_samples);
            });

            if let Some(ready_tx) = capture_ready_tx.take() {
                let _ = ready_tx.send(());
            }
        }
    }
}
