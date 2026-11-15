# Claude Code — Isle проєкт

Як працювати з цим проєктом:

1. Читайте файл `docs/PROMPT.md` (повний технічний промт із розділами 0–18)
2. Розділ 17 має пріоритет над усіма іншими розділами
3. Розробляйте за етапами (розд. 13): «Зроби Етап 1», «Етап 2» і т.д.
4. Перед завершенням кожного етапу виконайте навички:
   - `.claude/skills/perf-audit/SKILL.md`
   - Пройдіть чек-лист розділу 15
5. Усі рядки інтерфейсу — у `src/i18n/{uk,en,ru}.json` (жодних hard-coded рядків)
6. Код і коментарі — англійською, відповіді користувачеві — українською

Навігація:
- `src-tauri/src/main.rs` — вхід, всі IPC-команди
- `src-tauri/src/window.rs` — вікно острова, click-through, hit-region
- `src-tauri/src/settings.rs` — JSON-налаштування
- `src-tauri/src/db.rs` — SQLite (WAL)
- `src/lib/App.svelte` — головний компонент острова з вкладками
- `src/lib/modules/` — всі модулі (home, player, clipboard, capture, text, chat, notes, calendar, weather, control, phone, terminal, commandline, settings)

Сільски: `.claude/skills/` (6 файлів SKILL.md)