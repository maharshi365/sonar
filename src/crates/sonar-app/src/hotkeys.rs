use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

pub enum HotkeyAction {
    Transcribe,
    Cancel,
}

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    transcribe: HotKey,
    cancel: HotKey,
}

impl Hotkeys {
    pub fn new(transcribe: &str, cancel: &str) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|error| format!("failed to create global shortcut manager: {error}"))?;
        let transcribe = transcribe
            .parse::<HotKey>()
            .map_err(|error| format!("invalid transcribe shortcut: {error}"))?;
        let cancel = cancel
            .parse::<HotKey>()
            .map_err(|error| format!("invalid cancel shortcut: {error}"))?;
        manager
            .register_all(&[transcribe, cancel])
            .map_err(|error| format!("failed to register global shortcuts: {error}"))?;
        Ok(Self {
            manager,
            transcribe,
            cancel,
        })
    }

    pub fn poll(&self) -> Option<HotkeyAction> {
        let event = GlobalHotKeyEvent::receiver().try_recv().ok()?;
        if event.state != HotKeyState::Pressed {
            return None;
        }
        if event.id == self.transcribe.id() {
            Some(HotkeyAction::Transcribe)
        } else if event.id == self.cancel.id() {
            Some(HotkeyAction::Cancel)
        } else {
            None
        }
    }
}

impl Drop for Hotkeys {
    fn drop(&mut self) {
        let _ = self.manager.unregister_all(&[self.transcribe, self.cancel]);
    }
}
