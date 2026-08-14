//! Audio capture and processing.
//!
//! Ported from Handy's `audio_toolkit`, trimmed to what Sonar needs: cpal
//! capture, FFT-based resampling to 16 kHz, and a spectrum level meter. VAD was
//! intentionally dropped — transcribe-cpp's streaming path handles segmentation.

mod device;
mod recorder;
mod resampler;
mod visualizer;

pub use device::{list_input_devices, list_output_devices, CpalDeviceInfo};
pub use recorder::{
    is_microphone_access_denied, is_no_input_device_error, AudioFrameCallback, AudioRecorder,
    WHISPER_SAMPLE_RATE,
};
pub use resampler::FrameResampler;

/// Returns the appropriate cpal host for the current platform. On Linux, prefer
/// the ALSA host; elsewhere use the default.
pub fn get_cpal_host() -> cpal::Host {
    #[cfg(target_os = "linux")]
    {
        cpal::host_from_id(cpal::HostId::Alsa).unwrap_or_else(|_| cpal::default_host())
    }
    #[cfg(not(target_os = "linux"))]
    {
        cpal::default_host()
    }
}
