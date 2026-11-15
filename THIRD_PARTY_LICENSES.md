# Third-party licenses

Isle uses the following open-source dependencies (MIT / Apache-2.0 / BSD / OFL).

## Backend (Rust, `src-tauri/Cargo.toml`)

- tauri 2 — MIT/Apache-2.0
- tauri-plugin-global-shortcut — MIT/Apache-2.0
- tauri-plugin-single-instance — MIT/Apache-2.0
- tauri-plugin-notification — MIT/Apache-2.0
- tauri-plugin-dialog — MIT/Apache-2.0
- serde / serde_json — MIT/Apache-2.0
- rusqlite (bundled SQLite) — MIT; SQLite public domain
- reqwest — MIT/Apache-2.0
- chrono — MIT/Apache-2.0
- directories — MIT
- arboard — MIT/Apache-2.0
- winreg — MIT
- raw-window-handle — MIT/Apache-2.0
- base64 — MIT/Apache-2.0
- image — MIT/Apache-2.0
- qrcode — MIT/Apache-2.0
- rqrr — MIT/Apache-2.0
- uuid — MIT/Apache-2.0
- windows / windows-future — MIT/Apache-2.0
- portable-pty — MIT

## Frontend (npm, `package.json`)

- Svelte 5 — MIT
- Vite — MIT
- TypeScript — Apache-2.0
- @tauri-apps/api — MIT/Apache-2.0
- @tauri-apps/plugin-dialog — MIT/Apache-2.0
- @xterm/xterm — MIT
- @xterm/addon-fit — MIT

## Android companion

- AndroidX / Kotlin Gradle plugin — Apache-2.0
- Android framework APIs — Android SDK license

## Icons and fonts

- Custom island characters and SVG icons — original, © Isle contributors (MIT)
- Fonts (optional, OFL): Inter, Manrope, Nunito — SIL Open Font License 1.1

## Services (used only when enabled by the user)

- Open-Meteo — free non-commercial API (attribution required)
- LRCLIB — free lyrics source
- Ollama / LM Studio / llama.cpp / any OpenAI-compatible endpoint — user-controlled;
  no key or service is bundled with Isle
