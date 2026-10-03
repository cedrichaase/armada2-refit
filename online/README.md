# online — multiplayer over the internet, without port forwarding

Armada II's multiplayer is DirectPlay 8 underneath. That worked on a 2001 LAN. Over
today's internet it needs the host to forward ports or join a VPN, and under Proton it
does not work at all. This layer replaces DirectPlay with a transport of our own. Players
find each other through a small rendezvous server, connect directly where their routers
allow it, and fall back to a relay where they do not.

**State:** milestone 1 of the plan below. *Internet – Online* on the Multiplayer
Connection screen runs the game on `Online.asi`'s own transport, over UDP: host, join by
address, and play, under Proton without Microsoft's DirectPlay. There is no server yet,
so the host has to be reachable as on a LAN. Every other connection type keeps the
game's DirectPlay, traced but unchanged. `./install` does not install it yet.

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

## The menu entry

The Multiplayer Connection screen is a dialog of five `ShellButton`s: *Internet -
GameSpy*, *Internet - Manual IP*, *Local Area Network (IPX)*, *Local Area Network
(TCP/IP)* and *Previous Menu*. Each is labelled by `read_text_label("multiplayer_connection",
key)` from the game's `label.map`, and its click runs a branch of the dialog's
`WM_LBUTTONUP` case that ends in `GenericConnection(hwnd, lan, ipx, try_key, fail_key,
address)`. The Manual IP branch first runs `do_manual_ip` (the dialog with the address
field) and passes `manual_ip_address` with `lan` and `ipx` both 0, which is how the
Internet Game screen with *Create Game* comes up.

**The entry takes the IPX button's place.** IPX has not existed on Windows since Vista,
and neither Wine's DirectPlay nor Microsoft's can use it on any system this runs on, so
that button can only ever fail. Taking its slot keeps the screen's layout and art as they
are, where a fifth button would mean painting, hovering and hit-testing a control of our
own on top of the game's. LAN (TCP/IP) and Manual IP stay as they were.

| Address | Stock | Hook | Why |
|---|---|---|---|
| `0x4d9a30` | `read_text_label` | jump to `label_hook`, trampoline for the first 8 bytes | `lan_ipx` reads *Internet - Online*; while that entry is chosen, the Manual IP prompt (`multiplayer_manual_ip`), the connect/fail texts and the `commandline_net` titles read as online ones. None is longer than the stock text it replaces |
| `0x5bfd01` | the screen's `WM_LBUTTONUP` case | `stub_click` | clears the online flag on every click, so no other button inherits it |
| `0x5bff9b` | the IPX branch, its button just drawn pressed | `stub_ipx` | sets the flag and continues at `0x5c00b6`, the Manual IP branch after its own button: same frame, same stack depth, and that path sets every register it reads |

Each site is checked against the bytes it replaces, and if any differs none is patched
(`Online.log` says so). `Entry=0` in `Online.ini` leaves the screen stock.

The entry then runs the Manual IP flow (a blank field hosts, an address joins), and the
flag makes `NetworkManager.dll` get our transport instead of DirectPlay (next section). A
join code does not work yet: that is the server's, milestone 2.

## Our transport (milestone 1)

`peer.c` is an `IDirectPlay8Peer` of our own. When *Internet – Online* is the entry
chosen, `NetworkManager.dll`'s `CoCreateInstance` gets it instead of DirectPlay's, which
is never created, and the tracing proxy wraps it as before, so `Online.log` reads the same
for both and the two can be compared line by line. It implements the calls and messages
of the trace above, in the order DirectPlay delivered them. Addresses stay DirectPlay
address objects, which Wine does implement: we read `hostname` and `port` out of the
game's, and build the ones we hand it.

- **A star through the host.** A joiner talks to the host only, and the host forwards
  between joiners. Only the host then has to be reachable, which is the shape the
  server's hole punching and relay need next. The DPNIDs are ours.
- **One reliable, ordered stream per connection** over UDP: sequence numbers, a
  cumulative acknowledgement with a 32-packet selective bitmask, retransmission on a
  timer from the measured round trip, fragments of 1100 bytes. Every send is reliable
  and ordered whatever its flags; the game only ever asks for that. A connection that
  hears nothing for 15 s is lost, and a ping keeps quiet ones alive.
- **One network thread per peer** receives, retransmits and delivers every message to
  the game, in order. The game's calls only queue, and the game's handler is never
  called with our lock held, because the game calls back in from it.
- **Port 2302** (`Port=`), the port Armada II players already know to forward.

What the bench showed on the way, each now built in:

- **`Close` has to complete what is pending.** On quitting a match the game closes its
  session, creates a fresh peer, starts a search, and closes that one too. DirectPlay's
  `Close` ends the open search with `ASYNC_OP_COMPLETE (DPNERR_USERCANCEL)`, and the game
  cleans up on that message. Without it the game's main thread crashed on the way out
  (`Exception frame is not in stack limits`).
- **The game ages its game list by the provider's enum interval.** It passes 0 for the
  retry interval, takes the provider's default from `GetSPCaps`, and drops a listed game
  it has not heard from for a few multiples of it. With one datagram in ten lost, two
  missed round trips at DirectPlay's 1.5 s were enough for "The host of this game has
  been lost". Reporting 500 ms made it worse: the list then dropped games within a
  second and a half. So `GetSPCaps` reports DirectPlay's 1.5 s and the search actually
  queries every 500 ms.
- **`__stdcall` pops what it was declared with.** Every slot has its own exact argument
  count; one shared "return S_OK" for slots of different arity would unbalance the
  game's stack on the first call.

`Loss=N` in `Online.ini` drops N% of the datagrams a game sends, on purpose, for testing
on one machine, where nothing is ever lost. With `Loss=10` on both sides a match sent
about 960 datagrams each way, dropped about 100, resent 65 and 92, and played on.

## The plan

1. ~~Does multiplayer work under Proton?~~ No (above).
2. ~~A reference, and the trace.~~ Microsoft's DirectPlay on a bench clone, and
   `Online.asi` logging it (above). Still to trace: leaving, passwords, 3 players,
   voice.
3. ~~Two instances on the bench as a scenario.~~
   `./a2test run multiplayer-two-players` hosts, joins, chats both ways and plays a
   match, unattended, and checks both traces (below). Milestone 1 swaps its `Setup:`
   for our own transport and runs the same steps.
4. ~~The menu entry.~~ *Internet – Online* in the IPX button's place (below);
   `./a2test run multiplayer-online-entry` hosts and joins through it.
5. ~~Milestone 1:~~ our `IDirectPlay8Peer` over plain UDP (above).
   `./a2test run multiplayer-online-match` plays a whole 2-player match on it under plain
   Proton, and `multiplayer-online-loss` the same with 10% of datagrams dropped.
6. **Milestone 2:** the server, hole punching, relay fallback and join codes.
7. **Milestone 3:** the server-backed game list. **Milestone 4:** voice, if anyone wants it.

## Running the trace

On the bench, with two clones, using Microsoft's DirectPlay as the reference. Its DLLs are
not redistributable. `reference-dplay.sh` downloads the DirectX redistributable that
`winetricks directplay` uses into `$A2_DATA/reference/directx/` (checked against its
sha256) and installs the DLLs into a **clone's** prefix, never the real one.

Our own transport needs none of this: `online/bench-asi.sh` (a scenario's `Setup:`)
puts only `Online.asi` into a clone, and `multiplayer-online-match` and
`multiplayer-online-loss` run on it.

The reference, unattended, as a scenario (`testbench/scenarios/multiplayer-two-players.md`,
whose `Setup:` runs `reference-dplay.sh` on each clone):

    ./a2test run multiplayer-two-players --no-claude
    # each game's trace: <results>/multiplayer-two-players/1920x1080/<player>/logs/Online.log

By hand, to look around in between:

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
