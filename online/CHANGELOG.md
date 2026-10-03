# Changelog — online

`Online.asi`: online multiplayer without port forwarding, to replace the game's DirectPlay
8 with a transport of our own. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs
and versions". Newest first. Details are in [`README.md`](README.md).

## 0.2.0 — 2026-10-03

### Added
- *Internet – Online* on the Multiplayer Connection screen, in the IPX button's place:
  its dialog asks for a join code or the host's address (blank hosts), and for now it
  connects as Manual IP does. Key `Entry=` in `Online.ini` (1 by default; 0 leaves the
  screen stock) (fd40da4).

Installed, not yet seen in game.

## 0.1.0 — 2026-10-03

### Added
- `Online.asi`, trace only: it wraps the `IDirectPlay8Peer` that `NetworkManager.dll`
  creates and logs every call, message and address to `Online.log`. It changes nothing.
  Keys `Log=` and `Payload=` in `Online.ini` (14867b0).
- `install.sh` (and `--remove`). Not run by `./install`.
- `reference-dplay.sh`, which sets up a test-bench clone with Microsoft's DirectPlay
  (downloaded into `$A2_DATA/reference/directx/`) and `Online.asi`.

Installed, not yet seen in game.
