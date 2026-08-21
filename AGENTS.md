# Sonar

Cross-platform desktop speech-to-text app. Users press a global shortcut, speak, and the transcription is inserted into the active text field.

## Stack

- GPUI for the native desktop UI.
- Rust for audio processing, model management, and speech-to-text inference.
- A single native Rust process owns the UI and speech pipeline.

## Principles

- Keep audio and transcription fully local; never require cloud services.
- Make speech models easy to install, select, and hot-swap.
- Preserve consistent behavior across Windows, macOS, and Linux.
- Keep the interaction fast, simple, and privacy-focused.

## Workflow

- When using GitHub or other external code as a reference, prefer copying it into the repo first and then modifying it, rather than rewriting it from scratch.
