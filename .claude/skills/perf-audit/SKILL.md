---
name: perf-audit
description: Use before finishing any stage or when something feels slow; measures startup, idle CPU, RAM, frame time against the budgets.
---

- Budgets: start under 1.5 s, idle CPU under 1 percent, RAM 120 MB collapsed / 200 MB expanded.
- Measure: cold start, 10 min idle, tab switching, 100 clipboard items, music with lyrics.
- No polling: events only; stop timers when collapsed.
- Record numbers in PERF.md with date and machine.
- Fix regressions before moving to the next stage.