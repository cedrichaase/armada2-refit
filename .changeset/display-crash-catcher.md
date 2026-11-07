---
layers: [display]
bump: minor
status: Installed, not yet seen in game
---

### Added
- [Crash] Catch (on): the engine's crash handler no longer walks the call stack (which hangs under Wine and loses the exception); the first fatal exceptions are written to Display8.log (code, module+offset, registers, stack, frame walk) and the process ends with exit code 0xA2C0DE01. The address space (largest free block, free, committed) is logged at each device creation and reset, in the crash record and when it moves by 64 MB
