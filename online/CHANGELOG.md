# Changelog — online

`Online.asi`: online multiplayer without port forwarding, to replace the game's DirectPlay
8 with a transport of our own. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs
and versions". Newest first. Details are in [`README.md`](README.md).

## 0.5.0 — 2026-10-03

### Changed
- `Server=` defaults to `c20e.de`, the project's public server, in `Online.ini` and when
  the key is missing; `Server=` (empty) still means join by address only.
- `bench-asi.sh` empties `Server=` unless given `--server`: the bench never talks to the
  public server (f8012c9).

Confirmed in game 2026-10-03.

## 0.4.0 — 2026-10-03

### Added
- Join codes, hole punching and a relay, through `server/a2online-server.py` (new: the
  rendezvous and relay server, Python standard library only, with `selftest.py`). A host
  gets a code, shown in GAME SETUP's chat; a joiner types it where an address went. Key
  `Server=host:port` in `Online.ini` (empty by default: join by address as before), and
  `Direct=0` (testing: relay everything) (2f2255a).
- Lines in the game's chat boxes for the join code and for a code or server not found:
  two more checked call sites, the `Chat::Init` calls of the Internet Game screen and
  GAME SETUP.
- `bench-asi.sh --server` (a local server for the bench) and `--relay` (`Direct=0`).

Confirmed in game 2026-10-03.

## 0.3.0 — 2026-10-03

### Added
- Our own `IDirectPlay8Peer` over UDP (`peer.c`): a game started from *Internet –
  Online* runs on it instead of DirectPlay's, which under Proton cannot host. A star
  through the host, one reliable ordered stream per connection. Keys `Port=` (2302) and
  `Loss=` (testing: drop that percentage of datagrams) in `Online.ini` (6cbeed9).
- `bench-asi.sh`: `Online.asi` alone into a test-bench clone, with `--loss N`.

Confirmed in game 2026-10-03.

## 0.2.0 — 2026-10-03

### Added
- *Internet – Online* on the Multiplayer Connection screen, in the IPX button's place:
  its dialog asks for a join code or the host's address (blank hosts), and for now it
  connects as Manual IP does. Key `Entry=` in `Online.ini` (1 by default; 0 leaves the
  screen stock) (fd40da4).

Confirmed in game 2026-10-03.

## 0.1.0 — 2026-10-03

### Added
- `Online.asi`, trace only: it wraps the `IDirectPlay8Peer` that `NetworkManager.dll`
  creates and logs every call, message and address to `Online.log`. It changes nothing.
  Keys `Log=` and `Payload=` in `Online.ini` (14867b0).
- `install.sh` (and `--remove`). Not run by `./install`.
- `reference-dplay.sh`, which sets up a test-bench clone with Microsoft's DirectPlay
  (downloaded into `$A2_DATA/reference/directx/`) and `Online.asi`.

Confirmed in game 2026-10-03.
