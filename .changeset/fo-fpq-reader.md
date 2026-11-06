---
layers: [root]
bump: minor
---

### Added
- a2fmt: a reader for Fleet Operations: Roots' ODF.fpq (fpq, with its own LZO1X decoder in lzo), vfs::Fpq serving it as a mounted source, and a host tool (cargo run --example fpq).
