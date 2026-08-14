//! Receipt-sequenced Windows clipboard paste.
//!
//! A hidden message window publishes delayed `CF_UNICODETEXT`;
//! `WM_RENDERFORMAT` is the receipt proving that the focused application
//! consumed the transcript.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::borrow_as_ptr,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn,
    clippy::needless_pass_by_value,
    clippy::option_if_let_else,
    clippy::ptr_as_ptr,
    clippy::redundant_closure_call
)]

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc::Sender, Arc, Mutex, Once};
use std::thread;
use std::time::{Duration, Instant};

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    GlobalFree, SetLastError, ERROR_SUCCESS, HANDLE, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT,
    WPARAM,
};
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardOwner,
    GetClipboardSequenceNumber, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::System::Ole::{
    CF_BITMAP, CF_DSPBITMAP, CF_DSPENHMETAFILE, CF_DSPMETAFILEPICT, CF_DSPTEXT, CF_ENHMETAFILE,
    CF_OWNERDISPLAY, CF_PALETTE, CF_UNICODETEXT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CopyImage, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, KillTimer, PostQuitMessage, RegisterClassW, SetTimer, SetWindowLongPtrW,
    GDI_IMAGE_TYPE, GWLP_USERDATA, HWND_MESSAGE, IMAGE_FLAGS, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_DESTROYCLIPBOARD, WM_RENDERALLFORMATS, WM_RENDERFORMAT, WM_TIMER, WNDCLASSW,
};

use crate::transaction::{should_finish, State};

const CLASS_NAME: PCWSTR = w!("SonarPasteTransactionWindow");
const TIMER_ID: usize = 1;
const TIMER_INTERVAL_MS: u32 = 25;
const MAX_FORMAT_BYTES: usize = 64 * 1024 * 1024;
const CHORD_HOLD_MS: u64 = 100;
const IMAGE_BITMAP_TYPE: GDI_IMAGE_TYPE = GDI_IMAGE_TYPE(0);
const LR_CREATEDIBSECTION_FLAG: IMAGE_FLAGS = IMAGE_FLAGS(0x2000);

static TRANSACTION_ACTIVE: AtomicBool = AtomicBool::new(false);

struct SavedFormat {
    format: u32,
    data: Vec<u8>,
}

struct Shared {
    state: Mutex<State>,
    text: String,
    snapshot: Mutex<Vec<SavedFormat>>,
    saved_bitmap: Mutex<Option<isize>>,
    sequence: Mutex<u32>,
}

impl Drop for Shared {
    fn drop(&mut self) {
        if let Ok(bitmap) = self.saved_bitmap.get_mut() {
            if let Some(raw) = bitmap.take() {
                // SAFETY: A bitmap left here was duplicated by CopyImage and was
                // never transferred back to the clipboard.
                unsafe {
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(raw as *mut _));
                }
            }
        }
    }
}

struct ActiveGuard;

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        TRANSACTION_ACTIVE.store(false, Ordering::Release);
    }
}

enum ReliableError {
    Busy,
    Setup(String),
    Injection(String),
}

pub fn insert_text(text: &str) -> Result<(), String> {
    match reliable_paste(text) {
        Ok(()) => Ok(()),
        Err(ReliableError::Busy) => Err("a previous text insertion is still finishing".to_string()),
        Err(ReliableError::Injection(error)) => Err(error),
        Err(ReliableError::Setup(reliable_error)) => legacy_paste(text).map_err(|legacy_error| {
            format!(
                "reliable paste failed ({reliable_error}); fallback paste failed ({legacy_error})"
            )
        }),
    }
}

fn send_paste_chord() -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|error| format!("failed to initialize input injection: {error}"))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|error| format!("failed to press Control: {error}"))?;
    let click_result = enigo
        .key(Key::Other(0x56), Direction::Click)
        .map_err(|error| format!("failed to press V: {error}"));
    thread::sleep(Duration::from_millis(CHORD_HOLD_MS));
    let release_result = enigo
        .key(Key::Control, Direction::Release)
        .map_err(|error| format!("failed to release Control: {error}"));
    click_result.and(release_result)
}

fn reliable_paste(text: &str) -> Result<(), ReliableError> {
    if TRANSACTION_ACTIVE
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(ReliableError::Busy);
    }

    let shared = Arc::new(Shared {
        state: Mutex::new(State::new()),
        text: text.to_string(),
        snapshot: Mutex::new(Vec::new()),
        saved_bitmap: Mutex::new(None),
        sequence: Mutex::new(0),
    });
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let worker_shared = Arc::clone(&shared);
    thread::spawn(move || pump_thread(worker_shared, ready_tx));

    match ready_rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(ReliableError::Setup(error)),
        Err(_) => {
            return Err(ReliableError::Setup(
                "paste worker stopped before publishing".to_string(),
            ));
        }
    }

    if let Ok(mut state) = shared.state.lock() {
        state.injected_at = Some(Instant::now());
    }
    if let Err(error) = send_paste_chord() {
        if let Ok(mut state) = shared.state.lock() {
            state.injection_failed = true;
        }
        return Err(ReliableError::Injection(error));
    }
    Ok(())
}

fn legacy_paste(text: &str) -> Result<(), String> {
    enum SavedClipboard {
        Text(String),
        Image {
            width: usize,
            height: usize,
            bytes: Vec<u8>,
        },
        Empty,
    }

    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
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
    // SAFETY: This function has no preconditions and only reads system state.
    let sequence = unsafe { GetClipboardSequenceNumber() };
    drop(clipboard);

    thread::sleep(Duration::from_millis(40));
    let paste_result = send_paste_chord();
    thread::sleep(Duration::from_millis(500));

    // Do not overwrite content copied by the user while the target was pasting.
    // SAFETY: This function has no preconditions and only reads system state.
    if unsafe { GetClipboardSequenceNumber() } == sequence {
        let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
        match saved {
            SavedClipboard::Text(value) => clipboard
                .set_text(value)
                .map_err(|error| format!("failed to restore clipboard text: {error}"))?,
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
                .map_err(|error| format!("failed to restore clipboard image: {error}"))?,
            SavedClipboard::Empty => clipboard
                .clear()
                .map_err(|error| format!("failed to clear clipboard: {error}"))?,
        }
    }
    paste_result
}

unsafe fn shared_from_window(hwnd: HWND) -> *const Shared {
    // SAFETY: The window stores an Arc pointer for exactly the window lifetime.
    unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Shared }
}

unsafe fn render_text(shared: &Shared) {
    let wide_text: Vec<u16> = shared
        .text
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: Allocation size matches the UTF-16 buffer copied below.
    let Ok(memory) = (unsafe { GlobalAlloc(GMEM_MOVEABLE, wide_text.len() * 2) }) else {
        return;
    };
    // SAFETY: `memory` is a valid movable global allocation.
    let pointer = unsafe { GlobalLock(memory) } as *mut u16;
    if pointer.is_null() {
        // SAFETY: The allocation has not been transferred to the clipboard.
        let _ = unsafe { GlobalFree(Some(memory)) };
        return;
    }
    // SAFETY: Destination is at least `wide_text.len() * 2` bytes.
    unsafe { std::ptr::copy_nonoverlapping(wide_text.as_ptr(), pointer, wide_text.len()) };
    // SAFETY: `memory` is currently locked once.
    let _ = unsafe { GlobalUnlock(memory) };
    // SAFETY: The clipboard is open when required by the caller/WM_RENDERFORMAT.
    if unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(memory.0))) }.is_err() {
        // SAFETY: Ownership was not transferred when SetClipboardData failed.
        let _ = unsafe { GlobalFree(Some(memory)) };
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: The pointer is installed before the message pump starts.
    let shared = unsafe { shared_from_window(hwnd) };
    match message {
        WM_RENDERFORMAT => {
            if !shared.is_null() {
                // SAFETY: The Arc backing this pointer lives until window destruction.
                let shared = unsafe { &*shared };
                if let Ok(mut state) = shared.state.lock() {
                    state.receipts.push(Instant::now());
                }
                if wparam.0 as u32 == CF_UNICODETEXT.0 as u32 {
                    // SAFETY: Windows opens the clipboard for WM_RENDERFORMAT.
                    unsafe { render_text(shared) };
                }
            }
            LRESULT(0)
        }
        WM_RENDERALLFORMATS => {
            if !shared.is_null() {
                // SAFETY: The Arc backing this pointer lives until window destruction.
                let shared = unsafe { &*shared };
                // SAFETY: Win32 clipboard calls are balanced in this block.
                if unsafe { OpenClipboard(Some(hwnd)) }.is_ok() {
                    // SAFETY: Reading the current owner has no extra preconditions.
                    let still_owner = unsafe { GetClipboardOwner() }
                        .map(|owner| owner == hwnd)
                        .unwrap_or(false);
                    if still_owner {
                        // SAFETY: Clipboard is open and still owned by this window.
                        unsafe { render_text(shared) };
                    }
                    // SAFETY: Clipboard was opened successfully above.
                    let _ = unsafe { CloseClipboard() };
                }
            }
            LRESULT(0)
        }
        WM_DESTROYCLIPBOARD => {
            if !shared.is_null() {
                // SAFETY: The Arc backing this pointer lives until window destruction.
                if let Ok(mut state) = unsafe { &*shared }.state.lock() {
                    state.ownership_lost = true;
                }
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if !shared.is_null() {
                // SAFETY: The Arc backing this pointer lives until window destruction.
                on_timer(unsafe { &*shared });
            }
            LRESULT(0)
        }
        // SAFETY: Unhandled messages are delegated to the default window procedure.
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn ensure_window_class(instance: HINSTANCE) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        // SAFETY: The class structure and static class name outlive registration.
        unsafe {
            RegisterClassW(&class);
        }
    });
}

fn on_timer(shared: &Shared) {
    let finish = shared
        .state
        .lock()
        .map(|state| should_finish(&state, Instant::now()))
        .unwrap_or(true);
    if !finish {
        return;
    }

    let ownership_lost = shared
        .state
        .lock()
        .map(|state| state.ownership_lost)
        .unwrap_or(true);
    let sequence = shared.sequence.lock().map(|value| *value).unwrap_or(0);
    // SAFETY: This function has no preconditions and only reads system state.
    if !ownership_lost && unsafe { GetClipboardSequenceNumber() } == sequence {
        // SAFETY: The snapshot contains owned copies suitable for SetClipboardData.
        unsafe { restore_snapshot(shared) };
    }
    // SAFETY: Called on the worker's message-pump thread.
    unsafe { PostQuitMessage(0) };
}

unsafe fn snapshot_clipboard(hwnd: HWND, shared: &Shared) -> Result<(), String> {
    // SAFETY: Clipboard calls are balanced before every return.
    unsafe { OpenClipboard(Some(hwnd)) }
        .map_err(|error| format!("failed to open clipboard: {error}"))?;
    let result = (|| {
        let mut formats = Vec::new();
        let mut format = 0_u32;
        loop {
            // SAFETY: Enumerating from zero and then the previous result is the Win32 contract.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if format == CF_BITMAP.0 as u32 {
                // SAFETY: Clipboard is open and the requested format was enumerated.
                if let Ok(handle) = unsafe { GetClipboardData(format) } {
                    // SAFETY: CopyImage duplicates the clipboard-owned bitmap.
                    if let Ok(copy) = unsafe {
                        CopyImage(handle, IMAGE_BITMAP_TYPE, 0, 0, LR_CREATEDIBSECTION_FLAG)
                    } {
                        if let Ok(mut bitmap) = shared.saved_bitmap.lock() {
                            *bitmap = Some(copy.0 as isize);
                        }
                    }
                }
                continue;
            }
            if is_non_memory_format(format) {
                continue;
            }
            // SAFETY: Clipboard is open and the format was enumerated.
            if let Ok(handle) = unsafe { GetClipboardData(format) } {
                let memory = HGLOBAL(handle.0);
                // SAFETY: Non-global handles report zero and are skipped.
                let size = unsafe { GlobalSize(memory) };
                if size == 0 || size > MAX_FORMAT_BYTES {
                    continue;
                }
                // SAFETY: A nonzero global-memory clipboard handle can be locked for reading.
                let pointer = unsafe { GlobalLock(memory) } as *const u8;
                if pointer.is_null() {
                    continue;
                }
                // SAFETY: `pointer` references exactly `size` readable bytes.
                let data = unsafe { std::slice::from_raw_parts(pointer, size) }.to_vec();
                // SAFETY: The handle was locked once above.
                let _ = unsafe { GlobalUnlock(memory) };
                formats.push(SavedFormat { format, data });
            }
        }
        if let Ok(mut snapshot) = shared.snapshot.lock() {
            *snapshot = formats;
        }
        Ok(())
    })();
    // SAFETY: Clipboard was opened at function entry.
    let _ = unsafe { CloseClipboard() };
    result
}

fn is_non_memory_format(format: u32) -> bool {
    format == CF_ENHMETAFILE.0 as u32
        || format == CF_DSPENHMETAFILE.0 as u32
        || format == CF_DSPBITMAP.0 as u32
        || format == CF_DSPMETAFILEPICT.0 as u32
        || format == CF_DSPTEXT.0 as u32
        || format == CF_OWNERDISPLAY.0 as u32
        || format == CF_PALETTE.0 as u32
}

unsafe fn publish(hwnd: HWND) -> Result<u32, String> {
    // SAFETY: Clipboard calls are balanced before return.
    unsafe { OpenClipboard(Some(hwnd)) }
        .map_err(|error| format!("failed to open clipboard: {error}"))?;
    // SAFETY: Clipboard is open for this thread.
    let result = unsafe { publish_formats() };
    // SAFETY: Clipboard was opened above.
    let close_result = unsafe { CloseClipboard() };
    result?;
    close_result.map_err(|error| format!("failed to close clipboard: {error}"))?;
    // SAFETY: This function has no preconditions and only reads system state.
    Ok(unsafe { GetClipboardSequenceNumber() })
}

unsafe fn publish_formats() -> Result<(), String> {
    // SAFETY: Caller has opened the clipboard.
    unsafe { EmptyClipboard() }.map_err(|error| format!("failed to empty clipboard: {error}"))?;
    for (name, value) in [
        ("ExcludeClipboardContentFromMonitorProcessing", 1_u32),
        ("CanIncludeInClipboardHistory", 0_u32),
        ("CanUploadToCloudClipboard", 0_u32),
    ] {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is null-terminated and remains alive for this call.
        let format = unsafe { RegisterClipboardFormatW(PCWSTR(wide.as_ptr())) };
        if format == 0 {
            continue;
        }
        // SAFETY: Allocates one u32 for the marker value.
        if let Ok(memory) = unsafe { GlobalAlloc(GMEM_MOVEABLE, size_of::<u32>()) } {
            // SAFETY: `memory` is a valid allocation.
            let pointer = unsafe { GlobalLock(memory) } as *mut u32;
            if pointer.is_null() {
                // SAFETY: Ownership has not been transferred.
                let _ = unsafe { GlobalFree(Some(memory)) };
                continue;
            }
            // SAFETY: The allocation is exactly large enough for one u32.
            unsafe { *pointer = value };
            // SAFETY: The handle was locked once above.
            let _ = unsafe { GlobalUnlock(memory) };
            // SAFETY: Clipboard is open; success transfers allocation ownership.
            if unsafe { SetClipboardData(format, Some(HANDLE(memory.0))) }.is_err() {
                // SAFETY: Ownership was not transferred on failure.
                let _ = unsafe { GlobalFree(Some(memory)) };
            }
        }
    }

    // A null handle requests delayed rendering. windows-rs reports the null
    // success value as Err, so only a nonzero Win32 error is a real failure.
    unsafe { SetLastError(ERROR_SUCCESS) };
    // SAFETY: Clipboard is open and has an owner window.
    if let Err(error) = unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, None) } {
        if error.code().is_err() {
            return Err(format!("failed to publish delayed clipboard text: {error}"));
        }
    }
    Ok(())
}

unsafe fn restore_snapshot(shared: &Shared) {
    // SAFETY: Clipboard access is serialized by Windows.
    if unsafe { OpenClipboard(None) }.is_err() {
        return;
    }
    // SAFETY: Clipboard is open.
    let _ = unsafe { EmptyClipboard() };
    if let Ok(snapshot) = shared.snapshot.lock() {
        for saved in snapshot.iter().filter(|saved| !saved.data.is_empty()) {
            // SAFETY: Allocate exactly enough space for the saved bytes.
            let Ok(memory) = (unsafe { GlobalAlloc(GMEM_MOVEABLE, saved.data.len()) }) else {
                continue;
            };
            // SAFETY: `memory` is a valid allocation.
            let pointer = unsafe { GlobalLock(memory) } as *mut u8;
            if pointer.is_null() {
                // SAFETY: Ownership has not been transferred.
                let _ = unsafe { GlobalFree(Some(memory)) };
                continue;
            }
            // SAFETY: Destination was allocated to the exact source length.
            unsafe {
                std::ptr::copy_nonoverlapping(saved.data.as_ptr(), pointer, saved.data.len());
            }
            // SAFETY: The handle was locked once above.
            let _ = unsafe { GlobalUnlock(memory) };
            // SAFETY: Clipboard is open; success transfers allocation ownership.
            if unsafe { SetClipboardData(saved.format, Some(HANDLE(memory.0))) }.is_err() {
                // SAFETY: Ownership was not transferred on failure.
                let _ = unsafe { GlobalFree(Some(memory)) };
            }
        }
    }
    if let Ok(mut bitmap) = shared.saved_bitmap.lock() {
        if let Some(raw) = bitmap.take() {
            // SAFETY: The duplicated bitmap is now transferred to the clipboard.
            let result =
                unsafe { SetClipboardData(CF_BITMAP.0 as u32, Some(HANDLE(raw as *mut _))) };
            if result.is_err() {
                // SAFETY: Failed transfer leaves ownership with this process.
                let _ =
                    unsafe { DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(raw as *mut _)) };
            }
        }
    }
    // SAFETY: Clipboard was opened above.
    let _ = unsafe { CloseClipboard() };
}

fn pump_thread(shared: Arc<Shared>, ready: Sender<Result<(), String>>) {
    let _active_guard = ActiveGuard;
    // SAFETY: All Win32 resources created here are owned by this worker thread.
    unsafe {
        let instance = match GetModuleHandleW(PCWSTR::null()) {
            Ok(module) => HINSTANCE(module.0),
            Err(error) => {
                let _ = ready.send(Err(format!("failed to get module handle: {error}")));
                return;
            }
        };
        ensure_window_class(instance);
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CLASS_NAME,
            w!("SonarPasteTransaction"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                let _ = ready.send(Err(format!("failed to create paste window: {error}")));
                return;
            }
        };
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            Arc::into_raw(Arc::clone(&shared)) as *const _ as isize,
        );

        if let Err(error) = snapshot_clipboard(hwnd, &shared) {
            destroy_window(hwnd);
            let _ = ready.send(Err(error));
            return;
        }
        let sequence = match publish(hwnd) {
            Ok(sequence) => sequence,
            Err(error) => {
                // Publishing may have emptied the clipboard before failing.
                restore_snapshot(&shared);
                destroy_window(hwnd);
                let _ = ready.send(Err(error));
                return;
            }
        };
        if let Ok(mut saved_sequence) = shared.sequence.lock() {
            *saved_sequence = sequence;
        }
        if let Ok(mut state) = shared.state.lock() {
            state.published_at = Instant::now();
        }
        let _ = SetTimer(Some(hwnd), TIMER_ID, TIMER_INTERVAL_MS, None);
        let _ = ready.send(Ok(()));

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = DispatchMessageW(&message);
        }
        let _ = KillTimer(Some(hwnd), TIMER_ID);
        destroy_window(hwnd);
    }
}

unsafe fn destroy_window(hwnd: HWND) {
    // SAFETY: Pointer was created with Arc::into_raw and is reclaimed once.
    let shared = unsafe { shared_from_window(hwnd) };
    // SAFETY: Worker owns this message-only window.
    let _ = unsafe { DestroyWindow(hwnd) };
    if !shared.is_null() {
        // SAFETY: Reconstructs the one Arc consumed by Arc::into_raw.
        drop(unsafe { Arc::from_raw(shared) });
    }
}
