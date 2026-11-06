---
layers: [qol]
bump: minor
status: Installed, not yet seen in game
---

### Changed
- Move queues: each ship goes on to its next run (or the next part of a long one) the moment it reaches the end of its own; the group no longer waits at a run's end for its last ship (the 8 s straggler rule is gone). Ships still fly to their own formation places. `QOL.log`: `, N ship(s) behind` on a send.

### Fixed
- Move queues: the sends QOL makes on its own (next run, next part, places after a formation change) no longer play the ships' voice acknowledgement: `Craft::Say`'s call of `CaptainsVoice::Say` is re-pointed (entries `mq_say`, `mq_say_call`) and leaves the line out for those orders only, on this machine only. A Tab click's own sends are still spoken. `QOL.log`: ` (quiet)` on such a send, and `acknowledgement left out, queue K ship ID (topic T)`.
