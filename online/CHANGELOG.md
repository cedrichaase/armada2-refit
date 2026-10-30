# Changelog — online

`Online.asi`: online multiplayer without port forwarding, to replace the game's DirectPlay
8 with a transport of our own. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs
and versions". Newest first. Details are in [`README.md`](README.md).

## 0.7.1 — 2026-10-30

### Changed
- `online.c` (and `peer.c` through it) shed its copies of the helpers; the `.def` files moved to `csdk/def/`. Built on `csdk/a2asi.h` (the Win32 types, kernel32 imports, `s_*` string helpers, the append-a-line log, signature check and `VirtualProtect` patch) with the import libraries in `csdk/def/`; behaviour and log lines are unchanged.

Installed, not yet seen in game. (082d0c6)

## 0.6.3 — 2026-10-21

### Changed
- `Online.asi` carries a version resource (`VERSIONINFO`): product, description, the
  layer's version as the file version and the project's as the product version, which
  Windows shows in the file's Properties. Built by `platform/versioninfo/build-res.sh` (915cad8).

Confirmed in game 2026-10-22.

## 0.6.2 — 2026-10-17

### Fixed
- `bench-asi.sh --server`: the local server waits 600 s for its first datagram, not 90.
  Its idle time counts from the set-up, and a 4K take's games took about 100 s to reach
  their menus: the server was gone, the host had no join code, and the joiner typed a
  word of the error instead.

Installs nothing: a bench script.

## 0.6.1 — 2026-10-12

### Fixed
- `peer.c`: an `ENUM_HOSTS_RESPONSE` whose session name fills the datagram no longer
  hands the game a data size of -1 (`get_desc` counted a password byte that was not
  there), and responses are taken only from the address the search asks.
- `peer.c`: a reliable message is capped at 256 KB (`MAX_MSG`); a peer streaming
  fragments past it is dropped instead of growing the buffer without end. The buffer
  now grows geometrically.
- `peer.c`: a joiner ignores everything but `R_ACCEPT` before it is in, and fails the
  connect cleanly when its own player has no slot, where a host could crash it.
- `peer.c`: `DestroyPeer` closes the kicked joiner's connection to the game: nothing it
  sends is relayed (it went out as from DPNID 0), it no longer counts for the rules
  handshake, and the connection goes once the `R_TERMINATE` is acked or after 15 s.
- `peer.c`: `Close` or a rebind from inside the game's handler stops the network thread
  for good (a generation counter in place of `stop`), where the old thread kept running
  beside a new one.
- `peer.c`: every event allocation is checked; a joiner the game accepts but no slot is
  left for gets a `P_REJECT` (`SESSIONFULL`) instead of no answer.
- `online.c`: the menu entry is not patched if the trampoline cannot be made executable;
  the DirectPlay call log's line buffers fit four 399-character addresses.
- `server/a2online-server.py`: code lookups are rate-limited per IP address (2/s, burst
  10), unknown codes are logged in sum, not one line each, and a short LAN address no
  longer raises out of the handler. `selftest.py` checks both. The public server needs
  redeploying for this.

Installed, not yet seen in game.

## 0.6.0 — 2026-10-07

### Added
- Rules handshake (`R_RULES`): a joiner tells the host whether `QOLRules.asi` is loaded with
  `PayOnQueue=1`; the host answers every joiner, on every change, with whether every player
  has it, and calls `QOLRules_Network` / `QOLRules_Reset` in the plugin. Without it
  (or with a stock peer's missing plugin) the game plays by stock rules.

Confirmed in game 2026-10-09.

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
