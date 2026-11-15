# Будівництво Isle

## Вимоги
- Windows 10 22H2 або Windows 11
- Rust (stable) — https://rustup.rs/
- Node.js LTS — https://nodejs.org/
- WebView2 Runtime — https://developer.microsoft.com/microsoft-edge/webview2/
- NSIS (makensis) — для інсталятора (опціонально)
- SignTool + сертифікат — для підпису (опціонально)

## Збірка
```powershell
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process -Force
cd d:\island

# Розробка
.\build.ps1

# Реліз
.\build.ps1 -Release

# Реліз з підписом
.\build.ps1 -Release -Sign
```

## Вивід
- Бінар: `src-tauri/target/release/isle.exe` (фронтенд вбудований у бінар)
- Інсталятор Tauri: `src-tauri/target/release/bundle/nsis/Isle_1.6.0_x64-setup.exe`
- Portable: `Isle-portable.zip` (створює `build.ps1`) або просто скопіюйте `isle.exe`

## Автозапуск
У Налаштуваннях Isle увімкніть «Автозапуск із Windows». Програма додасться до `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` і запускатиметься разом із ПК.

## Етапи розробки
1. ✅ Каркас — вікно, анімація, годинник, погода, трей, i18n
2. ✅ Музика — SMTC, обкладинки, LRCLIB-тексти, керування, персонажі
3. ✅ Інструменти екрана — буфер, Alt+Space, GDI-скріншоти, OCR, QR
4. ✅ Продуктивність — текст, нотатки, таймер, нагадування, календар, ICS
5. ✅ Мережа й ШІ — чат, погода, Wi-Fi/VPN, адаптери
6. ✅ Телефон — Android-компаньйон, iPhone веб-версія, пейринг, сповіщення, файли
7. ✅ Агенти — Claude Code/Codex/OpenCode хуки, ConPTY-термінал
8. ✅ Полірування — інсталятор, документація, ліцензії, lazy-load, a11y
9. ✅ 1.6 — система віджетів, докінг, автобуфер, перекладач, OpenCode-плагін, анімації

## Відомі обмеження
- Запис екрана у відео не входить у 1.6; скріншоти з редактором/OCR/QR працюють.
- Плагінна система сторонніх розробників та автооновлення не входять у 1.6.
- FPS у грі потребує PresentMon/ETW і прав адміністратора.
- Температура CPU/GPU залежить від прошивки/драйвера або LibreHardwareMonitor.
- OCR потребує мовного пакета Windows для відповідної мови.
- Android-компаньйон збирається окремо в Android Studio.
- Підпис коду не налаштований: SmartScreen може блокувати непідписану збірку.