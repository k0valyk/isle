---
name: release-packaging
description: Use when building installers, signing, auto-update, or preparing a GitHub release.
---

- NSIS installer plus portable zip, under 20 MB; whisper models downloaded on demand.
- Sparse MSIX identity for notification access.
- Sign builds; document SmartScreen behavior for unsigned builds.
- Update through GitHub Releases; verify checksums.
- Update THIRD_PARTY_LICENSES.md on every dependency change.