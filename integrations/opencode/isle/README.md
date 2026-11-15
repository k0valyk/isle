# Isle plugin for OpenCode

Sends OpenCode activity to the Isle island: a green "working" banner while a
session runs, a "finished" banner on `session.idle`, and an attention banner
when a permission prompt appears.

## Install

1. Start Isle (it listens on `http://127.0.0.1:45871`).
2. Copy this folder into your OpenCode plugins directory:

   ```powershell
   New-Item -ItemType Directory -Force "$env:USERPROFILE\.config\opencode\plugins\isle" | Out-Null
   Copy-Item .\integrations\opencode\isle\index.ts "$env:USERPROFILE\.config\opencode\plugins\isle\index.ts" -Force
   ```

   Or use **Isle → Hooks → Install OpenCode plugin**, which writes the same file.
3. OpenCode reloads plugins automatically. Run `/reload` if it was already open.

The plugin is fire-and-forget: if Isle is closed, the requests fail silently and
OpenCode continues normally. No prompts or session data are stored by Isle; the
event title and a short event type are kept in memory only.
