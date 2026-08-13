# Sonar

Cross-platform desktop speech-to-text app. Users press a global shortcut, speak, and the transcription is inserted into the active text field.

## Stack

- Electron for the desktop UI.
- Rust for audio processing, model management, and speech-to-text inference.
- napi-rs to bridge Electron's Node main process with the Rust core (native addon, not a sidecar process).

## Principles

- Keep audio and transcription fully local; never require cloud services.
- Make speech models easy to install, select, and hot-swap.
- Preserve consistent behavior across Windows, macOS, and Linux.
- Keep the interaction fast, simple, and privacy-focused.
