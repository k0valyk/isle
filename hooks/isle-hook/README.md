# isle-hook

CLI-міст між агентами (Claude Code, Codex, OpenCode) та островом Isle.

## Збірка

```powershell
cd hooks\isle-hook
cargo build --release
```

Готовий файл: `hooks\isle-hook\target\release\isle-hook.exe`. Додай його теку до `PATH`
або викликай за повним шляхом.

## Використання

```powershell
isle-hook claude stop "Завдання виконано"
isle-hook codex notify "Чекаю на підтвердження"
isle-hook opencode build "Тести пройшли"
isle-hook --ping          # перевірити, чи запущений Isle
isle-hook --clear         # очистити стрічку подій
echo '{"source":"claude","event":"tool","title":"Bash","detail":"cargo test"}' | isle-hook
```

Події приймаються лише на `127.0.0.1:45871` і зберігаються в пам'яті Isle.

## Claude Code

Додай у `settings.json` (hooks):

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "isle-hook claude stop \"Task finished\"" }
        ]
      }
    ],
    "Notification": [
      {
        "hooks": [
          { "type": "command", "command": "isle-hook claude notify \"Needs attention\"" }
        ]
      }
    ]
  }
}
```

## Codex / OpenCode

Виклич `isle-hook codex <event> "<detail>"` з відповідного хука або обгортки.
Формат події вільний: `start`, `stop`, `notify`, `tool`, `error`, `done`.

## OpenCode (плагін)

У Isle → «Агенти» натисни «Встановити» або скопіюй `integrations/opencode/isle/index.ts` у
`%USERPROFILE%\.config\opencode\plugins\isle\index.ts`. Плагін сам надсилає `working`, `done`,
`error`, `attention`; Isle показує стан хука, стрічку подій і банери. Деталі — в
`integrations/opencode/isle/README.md`.
