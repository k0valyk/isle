# Isle — Live Island для Windows

Мета: безкоштовний, приватний аналог Dynamic Island для Windows 10/11 (Tauri 2 + Rust + Svelte 5).

Жорсткі умови:
- Безкоштовно: без підписок, ключів, реклами, платних API. Лише MIT/Apache-2.0/BSD залежності.
- Приватність: усе локально, без телеметрії та обов'язкових акаунтів.
- Бюджети: старт < 1.6 с, CPU idle < 1%, RAM 120 MB (згорнуто) / 200 MB (розгорнуто), ≥ 60 fps.

Правила коду: повний робочий код, без TODO/заглушок, без вигаданих API.

## Статус

Усі етапи реалізовано:

1. ✅ Каркас: вікно, анімація, годинник, погода, трей, i18n, налаштування
2. ✅ Музика: SMTC, обкладинки, LRCLIB-тексти, керування, реакція персонажа
3. ✅ Екран: буфер, Alt+Space, GDI-скріншоти, локальний OCR, QR-генерація/сканування
4. ✅ Продуктивність: текст, нотатки, таймер, нагадування, календар, ICS
5. ✅ Мережа й ШІ: локальний OpenAI-сумісний чат, погода, Wi-Fi/VPN/адаптери
6. ✅ Телефон: Android-застосунок, iPhone веб-версія, пейринг, сповіщення, буфер, файли
7. ✅ Агенти: HTTP-хуки Claude Code/Codex/OpenCode (`hooks/isle-hook`), ConPTY-термінал
8. ✅ Полірування: інсталятор, документація, ліцензії, lazy-load термінала, a11y/i18n

Окремо: острів можна перетягнути на будь-який монітор (позиція зберігається), є анімації
персонажа, появи вкладок, уваги до нагадувань і сповіщень.

Свідомо поза 1.6: відеозапис екрана, плагіни, автооновлення та код-підпис (див. BUILD.md).

## Збірка

```powershell
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process -Force
.\build.ps1          # розробка
.\build.ps1 -Release # реліз
.\build.ps1 -Release -Sign  # з підписом (потрібен signtool + сертифікат)
```

Release: `src-tauri\target\release\isle.exe`. Потрібна фіча `custom-protocol`
(`build.ps1` і Tauri CLI додають її автоматично).

## Архітектура

- `src-tauri/src/main.rs` — запуск, трей, хоткей, команди
- `media.rs` — SMTC-потік; `lyrics.rs` — LRCLIB; `capture.rs` — GDI + WinRT OCR + QR
- `productivity.rs` — календар/нагадування/ICS; `chat.rs` — OpenAI-сумісний клієнт
- `network.rs` — адаптери/Wi-Fi/VPN; `phone.rs` — LAN-сервер; `agents.rs` — hook-сервер
- `terminal.rs` — ConPTY; `db.rs` — SQLite-схема й команди; `settings.rs` — міграції
- `src/lib/modules/*` — вкладки острова; `src/SettingsApp.svelte` — налаштування

## Skills

`.claude/skills/`: windows-native-gotchas, island-motion, perf-audit, release-packaging,
android-companion, i18n-a11y.

## Відомі обмеження

- Код-підпис і автооновлення не налаштовані.
- Android-компаньйон збирається окремо в Android Studio.
- OCR залежить від мовних пакетів Windows.
- RemoteInput (відповіді на Android-сповіщення з ПК) не реалізовано.
