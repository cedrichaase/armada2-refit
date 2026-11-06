---
layers: [testbench]
bump: patch
status: none
---

### Fixed
- Scenario `engine-syms`: the QOL symbol count is 214 (was 212, stale since the move-queue merge). The other counts were checked against the profiles with `tools/engine/resolve.py` and already match; the scenario header now says they are the profile tables' sizes and must change with the profile.
