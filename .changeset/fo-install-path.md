---
layers: [root]
bump: minor
games: [armada2, fo-roots]
---

### Added
- games/<id>.toml 'installable' (./install and a2mod work on a game not offered); a requirement a game lacks is skipped; retire/ and move-keys run for armada2 only; a2mod stock on an unmodded game exits 0; layers.py --check and package.sh keep non-offered games out of the release and GUI.
