//! Input/output device enumeration and selection (cpal).
use cpal::traits::{DeviceTrait, HostTrait};
use std::collections::HashMap;

pub struct CpalDeviceInfo {
    pub index: String,
    pub name: String,
    pub is_default: bool,
    pub device: cpal::Device,
}

/// Lists the audio input devices visible to cpal.
///
/// # Errors
///
/// Returns an error when the platform audio host cannot enumerate input devices.
pub fn list_input_devices() -> Result<Vec<CpalDeviceInfo>, Box<dyn std::error::Error>> {
    let host = crate::get_cpal_host();
    let default_name = host.default_input_device().and_then(|d| d.name().ok());

    let mut out = Vec::<CpalDeviceInfo>::new();
    let mut name_counts = HashMap::<String, usize>::new();

    for device in host.input_devices()? {
        let name = device.name().unwrap_or_else(|_| "Unknown".into());
        let count = name_counts.entry(name.clone()).or_default();
        *count = count.saturating_add(1);
        let id = if *count == 1 {
            name.clone()
        } else {
            format!("{name}#{count}")
        };

        let is_default = Some(name.clone()) == default_name;

        out.push(CpalDeviceInfo {
            index: id,
            name,
            is_default,
            device,
        });
    }

    Ok(out)
}

/// Resolve a persisted device name to the current input device.
///
/// # Errors
///
/// Returns an error when input enumeration fails or `id` is not currently
/// available.
pub fn input_device_by_id(id: &str) -> Result<cpal::Device, Box<dyn std::error::Error>> {
    let devices = list_input_devices()?;
    devices
        .into_iter()
        .find(|info| info.index == id)
        .map(|info| info.device)
        .ok_or_else(|| {
            format!(
                "input device id '{id}' is unavailable; enumerate devices again and select a valid id"
            )
            .into()
        })
}

/// Lists the audio output devices visible to cpal.
///
/// # Errors
///
/// Returns an error when the platform audio host cannot enumerate output devices.
pub fn list_output_devices() -> Result<Vec<CpalDeviceInfo>, Box<dyn std::error::Error>> {
    let host = crate::get_cpal_host();
    let default_name = host.default_output_device().and_then(|d| d.name().ok());

    let mut out = Vec::<CpalDeviceInfo>::new();

    for (index, device) in host.output_devices()?.enumerate() {
        let name = device.name().unwrap_or_else(|_| "Unknown".into());

        let is_default = Some(name.clone()) == default_name;

        out.push(CpalDeviceInfo {
            index: index.to_string(),
            name,
            is_default,
            device,
        });
    }

    Ok(out)
}
