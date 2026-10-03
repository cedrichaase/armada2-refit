# online — multiplayer over the internet, without port forwarding

Armada II's multiplayer is DirectPlay 8 underneath. That worked on a 2001 LAN. Over
today's internet it needs the host to forward ports or join a VPN, and under Proton it
does not work at all. This layer replaces DirectPlay with a transport of our own. Players
find each other through a small rendezvous server, connect directly where their routers
allow it, and fall back to a relay where they do not.

**State:** step 2 of the plan below. `Online.asi` is a pass-through that only logs. It
changes nothing the game sends or receives, and `./install` does not install it yet.

## The goal

A player opens Multiplayer, picks **Internet – Online**, and either creates a game or
types a short join code. It just works: no port forwarding, no VPN, no client outside
the game. Linux and Windows players are treated the same.

### Decided (2026-10-03)

- **Server:** one public rendezvous/relay instance, run by the project, and the same
  program can be self-hosted. `Online.ini` names the server.
- **Our own protocol, no interop with players without the plugin.** In a LAN or
  GameRanger game, the plugin steps aside and the game's own DirectPlay runs.
- **A new menu entry** on the Multiplayer Connection screen, *Internet – Online*. Not
  the Manual IP path with a code typed into it.
- **Join codes first, then the game list.** The game's own Internet Game screen already
  has a *Current Games* list, so the lobby later means filling it from our server.
- **No code from DirectPlay Lite.** That reimplementation is GPL-2.0 and this repository
  is MIT, so it is a behavioural reference at most. The DirectPlay subset is written
  from Microsoft's documented API and from what this layer's trace shows.

### Architecture

1. **`Online.asi`, the client.** `NetworkManager.dll` creates DirectPlay through a
   single `CoCreateInstance` (below). The plugin hands it our own `IDirectPlay8Peer`,
   only the subset the game calls. Underneath is UDP with a small reliability layer.
   Players are addressed by server-issued IDs, not IP addresses. Connections are
   hole-punched where possible and relayed where not.
2. **A rendezvous and relay server.** It reports each player's public address, introduces
   peers to each other for hole punching, relays when punching fails, and later lists
   open games.
3. **The menu entry and the join code,** inside the game's own shell screens.

The level below DirectPlay, the sockets, was rejected as the place to intercept. It would
keep Proton's DirectPlay, which cannot host (below), and DirectPlay writes addresses into
its own packets.

## What is established

### Proton's DirectPlay cannot host: multiplayer is broken on Linux today

With the bench's `A2TEST_WINEDEBUG=err+all,fixme+dpnet,trace+dpnet`, Create Game ends in
"Error! Cannot connect to Manual IP". Wine's builtin `dpnet.dll` reports
`IDirectPlay8PeerImpl_Host … stub` and `IDirectPlay8PeerImpl_EnumHosts … stub`
(Proton-CachyOS 11.0, 2026-10-03). LAN games are broken too, so on Linux our transport
is a requirement, not just an improvement.

With **Microsoft's** DirectPlay in the prefix (below), everything works on the bench:
two instances on one machine host, find the game, join, chat, and play a match.

### Where DirectPlay is created

`Armada2.exe` imports only `CoCreateGuid` from ole32. `NetworkManager.dll` imports
`CoCreateInstance`, so its one import-table slot sees every DirectPlay object the game
creates: an `IDirectPlay8Peer` (`CLSID_DirectPlay8Peer`), never Client/Server, and
`IDirectPlay8Address` objects. `NetworkManager.dll` also references DirectPlay Voice.
The voice classes were not created in the trace below; that is still to be checked
with voice switched on.

### What the game calls (trace, 2026-10-03)

The trace below is from two bench instances on one machine: host plus joiner, chat, and
about 2.5 minutes of a 2-player match on Warzone. Across both sides the whole surface is:

| Calls | Messages |
|---|---|
| `Initialize`, `GetSPCaps`, `EnumHosts`, `CancelAsyncOperation`, `SetPeerInfo`, `GetPeerInfo`, `Host`, `Connect`, `SendTo` | `ENUM_HOSTS_QUERY`/`_RESPONSE`, `ASYNC_OP_COMPLETE`, `INDICATE_CONNECT`, `CONNECT_COMPLETE`, `CREATE_PLAYER`, `RECEIVE`, `SEND_COMPLETE`, `RETURN_BUFFER` |

The trace stops with both instances still in the match, so leaving is not in it.

- **Finding games:** `EnumHosts` runs forever (count and timeout −1), and the game
  cancels it when the player picks one. The host's game handler answers each query with
  a 74-byte reply, the game's own summary. The game shows `name (1/8) 1ms "Warzone"`
  from the reply data and the session description.
- **Addresses:** the only address the game builds is the one the player types. Manual IP
  "10.0.0.19" becomes `hostname=10.0.0.19` in `EnumHosts`, then the responder's
  `hostname=…;port=2302` goes into `Connect`. The game **never asks DirectPlay for a
  peer's address**: no `GetPeerAddress`, no `GetLocalHostAddresses`. The "Local IP
  Address" line comes from `NetworkManager.dll`'s own Winsock calls. So outside the
  `hostname` string the game types, addresses are opaque to it, and our transport is
  free to use whatever IDs it likes.
- **Names:** `SetPeerInfo` with the player's name before hosting or connecting, and
  `GetPeerInfo` (asking for the size first) on every `CREATE_PLAYER`.
- **Session:** `Host` with max 8 players and the game's name. There is no
  application-reserved data, and a password field when one is set. `Connect` sends no
  user data.
- **Traffic:** every `SendTo` is `DPNSEND_GUARANTEED` (0x08), asynchronous, ordered,
  normal priority, to one player, **itself included**. Its own sends come back as
  `RECEIVE`. The one exception is a 31-byte message the joiner sends to
  `DPNID_ALL_PLAYERS_GROUP` right after `CONNECT_COMPLETE`. Every payload starts with
  `0x7f`.
- **Rate:** in the match, each player sends one message to each player every ~233 ms,
  about 11 bytes each. That is 8.6 sends and 94 payload bytes per second per player in
  a 2-player game. The setup screen sends 481-byte messages. A relay for a full 8-player
  game would carry well under 50 KB/s.

### Not yet established

- Leaving, dropping, and the host quitting: `DestroyPlayer`, `TerminateSession`, host
  migration.
- Passwords, and 3 or more players. Two peers do not show how a full mesh is built.
- DirectPlay Voice.
- Whether a refit install and a stock install can share a game (the game compares CRCs).

## The plan

1. ~~Does multiplayer work under Proton?~~ No (above).
2. ~~A reference, and the trace.~~ Microsoft's DirectPlay on a bench clone, and
   `Online.asi` logging it (above). Still to trace: leaving, passwords, 3 players,
   voice.
3. **Two instances on the bench as a scenario,** so host and join can be checked without
   a human. Today it is the manual procedure below.
4. **The menu entry:** *Internet – Online* on the Multiplayer Connection screen.
5. **Milestone 1:** our `IDirectPlay8Peer` over plain UDP on a LAN, with a whole 2-player
   match on the bench.
6. **Milestone 2:** the server, hole punching, relay fallback and join codes.
7. **Milestone 3:** the server-backed game list. **Milestone 4:** voice, if anyone wants it.

## Running the trace

On the bench, with two clones, using Microsoft's DirectPlay as the reference. Its DLLs are
not redistributable. `reference-dplay.sh` downloads the DirectX redistributable that
`winetricks directplay` uses into `$A2_DATA/reference/directx/` (checked against its
sha256) and installs the DLLs into a **clone's** prefix, never the real one:

    ./a2test session start --res 16:9 --no-launch      # twice: host and joiner
    online/reference-dplay.sh <state.json of each>     # Microsoft DirectPlay + Online.asi
    A2TEST_SESSION=<host state.json> ./a2test drive launch
    # host: Multiplayer emblem > Internet - Manual IP > O.K. with the field empty >
    #       Create Game > O.K.
    # joiner: rename at design 400,187 > Internet - Manual IP > design 400,292,
    #         type the host's "Local IP Address" > O.K. > click the game > Join Game
    # LAUNCH on the host, then on the joiner, then on the host again
    # the logs: <clone>/game/Online.log

Traps found on the way:

- **`a2test drive` drives the newest session** unless `A2TEST_SESSION` names one, so set
  it on every command when two are running.
- **The Multiplayer button is the emblem, not the word.** `click-text "Multiplayer"`
  misses; the scenario step "Click the multiplayer emblem." hits.
- **A clone can start with no Wine desktop.** The log shows `failed to start explorer`
  and the screen stays black. `drive quit` and `drive launch` again.
- **Launching takes three presses:** the host asks, each joiner confirms (its light
  turns green), and the host launches.
