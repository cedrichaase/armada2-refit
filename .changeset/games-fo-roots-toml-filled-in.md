---
layers: [root]
bump: minor
---

### Added
- games/fo-roots.toml filled in from the install FO-0 ran (folder and prefix names, Settings.xml under Data/settings_saves_logs, the crash handler's file); game descriptors gain a `[logs]` table (`crash`, `dirs`), which tools/layers.py --check requires
