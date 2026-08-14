#![allow(clippy::option_if_let_else)]

use std::borrow::Cow;
use std::thread;
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use objc2_app_kit::NSPasteboard;

const PASTE_DELAY: Duration = Duration::from_millis(40);
const RESTORE_DELAY: Duration = Duration::from_millis(500);
const CHORD_HOLD: Duration = Duration::from_millis(100);

enum SavedClipboard {
    Text(String),
    Image {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    },
    Empty,
}

pub fn insert_text(text: &str) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("failed to open clipboard: {error}"))?;
    let saved = if let Ok(value) = clipboard.get_text() {
        SavedClipboard::Text(value)
    } else if let Ok(image) = clipboard.get_image() {
        SavedClipboard::Image {
            width: image.width,
            height: image.height,
            bytes: image.bytes.into_owned(),
        }
    } else {
        SavedClipboard::Empty
    };

    clipboard
        .set_text(text)
        .map_err(|error| format!("failed to write transcript to clipboard: {error}"))?;
    let change_count = NSPasteboard::generalPasteboard().changeCount();
    drop(clipboard);

    thread::sleep(PASTE_DELAY);
    let paste_result = send_paste_chord();
    thread::sleep(RESTORE_DELAY);

    // A changed count means the user or another application replaced the
    // clipboard while the paste was in flight; their content must win.
    if NSPasteboard::generalPasteboard().changeCount() == change_count {
        restore_clipboard(saved)?;
    }
    paste_result
}

fn send_paste_chord() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|error| {
        format!(
            "failed to initialize input injection; grant Sonar Accessibility permission: {error}"
        )
    })?;
    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(accessibility_error)?;
    // Physical key code 9 is V on macOS and remains correct across layouts.
    let click_result = enigo
        .key(Key::Other(9), Direction::Click)
        .map_err(accessibility_error);
    thread::sleep(CHORD_HOLD);
    let release_result = enigo
        .key(Key::Meta, Direction::Release)
        .map_err(accessibility_error);
    click_result.and(release_result)
}

fn accessibility_error(error: impl std::fmt::Display) -> String {
    format!("failed to send paste shortcut; grant Sonar Accessibility permission: {error}")
}

fn restore_clipboard(saved: SavedClipboard) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("failed to reopen clipboard: {error}"))?;
    match saved {
        SavedClipboard::Text(value) => clipboard
            .set_text(value)
            .map_err(|error| format!("failed to restore clipboard text: {error}")),
        SavedClipboard::Image {
            width,
            height,
            bytes,
        } => clipboard
            .set_image(ImageData {
                width,
                height,
                bytes: Cow::Owned(bytes),
            })
            .map_err(|error| format!("failed to restore clipboard image: {error}")),
        SavedClipboard::Empty => clipboard
            .clear()
            .map_err(|error| format!("failed to clear clipboard: {error}")),
    }
}
