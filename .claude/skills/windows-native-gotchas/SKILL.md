---
name: windows-native-gotchas
description: Use when writing Win32/COM/Tauri window code for the island (transparent no-focus window, click-through, hotkeys, hooks, DPI, SMTC, Core Audio, UserNotificationListener).
---

- Window: WS_EX_TOOLWINDOW + WS_EX_NOACTIVATE, topmost, Per-Monitor DPI V2.
- Click-through: update hit-region only when shape changes; keep a few px margin.
- Hotkeys: check RegisterHotKey result, surface conflicts, offer fallbacks.
- Hooks: disable LL keyboard hook in fullscreen games, fall back to GetKeyState.
- Notifications need package identity (sparse MSIX), otherwise disable feature.
- Never block the UI thread; COM objects live on their own threads.