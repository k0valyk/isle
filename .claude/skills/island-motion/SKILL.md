---
name: island-motion
description: Use when building or tuning island animations (morphing, spring physics, wavy edge, characters, toasts, reduced motion).
---

- Animate only transform, opacity, clip-path; no layout during morph.
- Spring defaults: stiffness 300, damping 28; expose in settings.
- Wavy edge: 64 control points, spring + noise, amplitude from FFT.
- Characters share states: idle, follow, dance, think, happy, alert, sleep.
- Respect prefers-reduced-motion and the Less animations toggle.
- Target 60 fps minimum, 120 on 120 Hz; frame under 8 ms.