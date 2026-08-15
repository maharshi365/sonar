//! Cross-application text insertion.
//!
//! Windows uses delayed clipboard rendering so the previous clipboard is only
//! restored after the focused application has actually read the transcript.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod transaction;
#[cfg(windows)]
mod windows;

/// Insert text into the currently focused application.
///
/// # Errors
///
/// Returns an error when the platform is unsupported or input injection fails.
#[allow(clippy::needless_return)] // Each return is selected at compile time by cfg.
pub fn insert_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    #[cfg(windows)]
    {
        return windows::insert_text(text);
    }

    #[cfg(target_os = "macos")]
    {
        return macos::insert_text(text);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    Err("text insertion is not implemented on this platform yet".to_string())
}

/// Send a submit shortcut to the currently focused application.
///
/// # Errors
///
/// Returns an error when input injection is unsupported or fails.
#[allow(clippy::needless_return)]
pub fn submit(key: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        return windows::submit(key);
    }

    #[cfg(target_os = "macos")]
    {
        return macos::submit(key);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    Err(format!(
        "submit shortcut '{key}' is not implemented on this platform yet"
    ))
}
