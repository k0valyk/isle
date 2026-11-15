# Isle — карта проєкту

Коротко: Tauri 2 (Rust) + Svelte 5. Острів — прозоре вікно `main` (600×400), регіон `SetWindowRgn`.
Налаштування — `%APPDATA%\Isle\settings.json`, БД — `%APPDATA%\Isle\isle.db`.

## Rust (`src-tauri/src`)
| Файл | Відповідає за |
|---|---|
| `main.rs` | setup, tray, hotkeys, команди, докінг, кастомний drag, реєстрація команд |
| `window_util.rs` | регіон вікна, work area, позиція, dock, fullscreen |
| `settings.rs` | схема налаштувань (schema 3), міграції, DPAPI для AI-ключа |
| `media.rs` | SMTC-сесії, обкладинка, play/pause/seek, `media_current` |
| `lyrics.rs` | LRCLIB: нормалізація метаданих, synced/plain |
| `artwork.rs` | фолбек обкладинки iTunes/Deezer |
| `audio.rs` | WASAPI loopback + FFT-спектр |
| `stats.rs` | CPU/GPU/RAM/диск/мережа/температури/частота |
| `db.rs` | SQLite: буфер (текст+зображення), нотатки, автowatcher |
| `translate.rs` | переклад MyMemory |
| `capture.rs` | скріншоти, OCR, QR, збереження |
| `chat.rs` | OpenAI-сумісний чат |
| `agents.rs` | HTTP-хуки, статус, OpenCode-плагін |
| `phone.rs`, `phone_web.html` | LAN-пейринг, iPhone-сторінка |
| `downloads.rs`, `network.rs`, `productivity.rs`, `terminal.rs`, `battery.rs`, `weather.rs`, `hotkeys.rs`, `autostart.rs`, `cursor.rs` | відповідні модулі |

## Frontend (`src`)
| Файл | Відповідає за |
|---|---|
| `IslandApp.svelte` | острів: регіон, докінг, drag, режими, банери, анімації, auto lyrics |
| `SettingsApp.svelte` | вікно налаштувань (секції) |
| `lib/store.svelte.ts` | settings store, теми, i18n |
| `lib/media.svelte.ts` | медіа/лірика/обкладинка/accent |
| `lib/timer.svelte.ts` | таймер (спільний для модуля й острова) |
| `lib/stats.svelte.ts`, `battery.svelte.ts`, `agents.svelte.ts`, `clock.svelte.ts` | відповідні stores |
| `lib/modules/*` | вкладки: home, player, clipboard, text, notes, timer, weather, capture, calendar, chat, control, stats, agents, phone, terminal |
| `lib/widgets/*` | система віджетів |
| `lib/components/*` | Icon, Clock, Character, MiniSpectrum, WeatherBadge, BatteryBadge, HotkeyInput |
| `i18n/{uk,en,ru}.json` | усі рядки (478 ключів) |
| `app.css` | глобальні стилі/прозорість |

## Ключові факти
- Release: `npx.cmd tauri build --no-bundle` або `cargo build --release --features custom-protocol`.
- Інсталятор: `npx.cmd tauri bundle` → `target/release/bundle/nsis/Isle_1.6.0_x64-setup.exe`.
- Тести: `cargo clippy`, `npm.cmd run check`, i18n parity.
- CDP: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=NNNN`.
- Білі артефакти: регіон точно = острів; WebView2 background; перевіряти `SetWindowRgn`.
