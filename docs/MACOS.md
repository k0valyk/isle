# Isle для macOS — план порту

## Головне обмеження

- **Зібрати `.app`/`.dmg` можна лише на Mac** (Xcode Command Line Tools + Rust target
  `aarch64-apple-darwin`/`x86_64-apple-darwin`). З Windows крос-компіляція Tauri на macOS
  неможлива: немає macOS SDK, код-підпису й `hdiutil`.
- Поточна версія — **Windows-native**: 22 модулі Rust використовують Win32/WinRT
  (`SetWindowRgn`, SMTC, WASAPI, GDI, PDH, DPAPI, ConPTY, `PlaySoundW`).
- Візуальна частина (Svelte 5) і більшість бізнес-логіки — кросплатформні, їх переносити не треба.

## Що вже кросплатформне (працює як є)

`agents`, `artwork` (мережа), `chat`, `db` (SQLite), `downloads`, `lyrics` (LRCLIB), `network`,
`phone` (HTTP), `productivity`, `translate`, `weather`, `settings` (схема/JSON),
весь фронтенд, i18n, віджети, таймер-стан, буфер обміну (`arboard`), QR/OCR-залежності.

## Що треба замінити (модуль → macOS-бекенд)

| Модуль | Windows | macOS |
|---|---|---|
| `window_util` | `SetWindowRgn`, `SetWindowPos`, DPI | `NSWindow` borderless + `isOpaque = false`, `backgroundColor = .clear`, `level = .statusBar`, `collectionBehavior = [.canJoinAllSpaces, .stationary]`, `ignoresMouseEvents` для click-through. Форма — нативна прозорість, `SetWindowRgn` не потрібен |
| `media` | SMTC | `MediaRemote` (`MRMediaRemoteGetNowPlayingInfo`) або AppleScript до Music/Spotify; обкладинка з `MPNowPlayingInfoPropertyAssetURL` / скрипта застосунку |
| `audio` (FFT) | WASAPI loopback | CoreAudio process tap (macOS 14.4+) або `ScreenCaptureKit` audio; альтернатива — віртуальний пристрій BlackHole |
| `capture` | GDI BitBlt | ScreenCaptureKit `SCScreenshotManager` + дозвіл «Запис екрана» (`CGPreflightScreenCaptureAccess`) |
| `stats` | PDH + GetSystemTimes | `sysinfo` crate; GPU — IOKit `IOAccelerator`; температури — SMC (IOKit) |
| `sound` | `PlaySoundW` | `rodio`/`cpal` (кросплатформні) або `afplay` |
| `terminal` | ConPTY | `portable-pty` уже кросплатформний, шел — `/bin/zsh` |
| `autostart` | реєстр Run | LaunchAgent `~/Library/LaunchAgents/com.isle.app.plist` |
| `battery` | Win32 | `pmset -g batt` / IOKit |
| `cursor` | `GetCursorPos` | `CGEvent(source: nil).location` |
| `hotkeys` | RegisterHotKey | `tauri-plugin-global-shortcut` (потрібен дозвіл Accessibility) |
| `settings` (ключ) | DPAPI | Keychain (`security` crate) |
| `phone`/`notifications` | WinRT listener | `tauri-plugin-notification` (уже кросплатформний), телефон — той самий HTTP |

## Пакет і розповсюдження

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm ci
npm run tauri build -- --target universal-apple-darwin
# → src-tauri/target/universal-apple-darwin/release/bundle/dmg/Isle_1.6.0_universal.dmg
```

- Без Apple Developer ID ($99/рік) застосунок не підписаний і не нотаризований:
  користувач відкриває через «Правий клік → Відкрити» або `xattr -dr com.apple.quarantine /Applications/Isle.app`.
- Мінімум macOS 13 (CoreAudio tap — 14.4; без нього FFT-візуалізація недоступна).

## Обсяг робіт

| Етап | Що входить | Оцінка |
|---|---|---|
| 1. Каркас | `#[cfg]`-розділення модулів, `platform/` шар, macOS-заглушки, збірка `.app` | 2–3 дні |
| 2. Ядро | острів (прозоре вікно, регіон, докінг, drag), таймер, переклад, нотатки, буфер, погода, чат, налаштування | 3–4 дні |
| 3. Системне | скріншоти (ScreenCaptureKit), медіа (MediaRemote), звук таймера, батарея, автозапуск, хоткеї | 4–5 днів |
| 4. Поліш | FFT-візуалізація (CoreAudio tap), статистика/температури, CI, підпис | 3–5 днів |

**Разом:** 2–3 тижні роботи + Mac для перевірки. Windows-версія при цьому не ламається,
якщо тримати платформні реалізації за `cfg(windows)` і `cfg(target_os = "macos")`.

## CI (коли буде git-репозиторій)

`.github/workflows/macos.yml` уже доданий: збирає universal `.dmg` на `macos-14`,
завантажує артефакт. Локально на Windows його запустити неможливо.
