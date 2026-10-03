# online — multiplayer over the internet, without port forwarding

Armada II's multiplayer is DirectPlay 8 underneath. That worked on a 2001 LAN. Over
today's internet it needs the host to forward ports or join a VPN, and under Proton it
does not work at all. This layer replaces DirectPlay with a transport of our own. Players
find each other through a small rendezvous server, connect directly where their routers
allow it, and fall back to a relay where they do not.

**State:** milestone 2 of the plan below. *Internet – Online* on the Multiplayer
Connection screen runs the game on `Online.asi`'s own transport, over UDP, under Proton
without Microsoft's DirectPlay. Through the server named in `Online.ini` (the public one
at `c20e.de` unless changed), the host gets a join code and players type it to join:
directly where the routers allow it, through the server's relay where not. With
`Server=` empty, players join by address as on a LAN. The server is
`server/a2online-server.py`. Every other connection type keeps the game's DirectPlay,
traced but unchanged. `./install` does not install it yet.

**Seen in game (2026-10-03):** a player's own install joined a bench game on the same
machine by join code, through the public server at `c20e.de`, and played the match. That confirms the menu entry, the transport, the join code and the default
server. It does **not** cover two networks: on one machine the direct path always wins,
so neither hole punching through a real router nor the relay has been seen outside the
bench ("Not yet established" under milestone 2).

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
The voice classes were not created in the trace below. Voice is not part of this layer
(see "The plan"); what remains is to check that an online game survives a player
switching the game's voice on.

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
- What the game does with voice switched on (only that it must not break an online
  game; voice itself is out of scope).
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

The entry then runs the Manual IP flow (a blank field hosts, a join code or an address
joins), and the flag makes `NetworkManager.dll` get our transport instead of DirectPlay
(next section).

Two more call sites belong to the entry, for the server's messages to the player (below,
"Showing the join code"). They are checked and patched with the rest:

| Address | Stock | Hook | Why |
|---|---|---|---|
| `0x5b2ff2` | `InternetGameDlgProc`: `chatRoom.Init(GetDlgItem(...))` | `call stub_room_init` | the same call, then `on_chat_init`: a line waiting for the Internet Game screen ("No game with the join code …") goes in |
| `0x5c5395` | `MultiplayerSetupDlgProc`: `chatGame.Init(...)` | `call stub_game_init` | the same for GAME SETUP, where the host's join code is posted again every time the screen opens |

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

## The server and join codes (milestone 2)

`server/a2online-server.py` is the rendezvous and relay server: one UDP port (2399), Python
3.8 and its standard library, nothing on disk. `Server=host:port` in `Online.ini` names
it. The client speaks to it from the game's own socket, so the address the server sees
for a player is that player's NAT mapping for the socket the game talks on, which is
what the other player needs. The messages are `SV_*` in `peer.c` and the server, after
the same `A2O` header as the rest of the protocol:

- **Hosting.** `Host` sends `SV_HOST` with a random token and the host's LAN address,
  and again every 10 s. The server answers `SV_HOSTED` with a join code, the public
  address it sees, and an id for the relay. The token keeps the code if the host's
  router gives it a new port. A code is six characters from 31 that cannot be misread
  (no I, L, O, 0, 1), shown as `K7M-Q2X`; a host silent for 40 s, or its `SV_BYE` from
  `Close`, frees it.
- **Joining.** A join code typed into the address field reaches `EnumHosts` as the
  address's `hostname`; six code characters (any case, dashes ignored) are taken as a
  code, anything else as an address as before. `SV_JOIN` gets `SV_PEER` back with the
  host's public and LAN address, and the host gets `SV_INTRO` with the joiner's, plus a
  shared pair token. An unknown code gets `SV_NOTFOUND`.
- **The direct path.** For 2.5 s both send `P_PUNCH` probes to every address they have
  for the other, every 200 ms, and answer the probes that arrive. Each side's probes
  open its own router to the other's, so where routers keep one public port per socket
  (most home routers) the probes meet. The joiner takes the first address an *answer*
  comes from, a round trip that proves both directions. It also probes back any address
  a probe arrived from, which covers a host whose router picked a new port. Two players
  behind the same router reach each other on their LAN addresses.
- **The relay.** With no answer after 2.5 s, the joiner goes through the server.
  `SV_RELAY [to id][datagram]` arrives at the other side as `SV_RELAYED [from id]
  [datagram]`. The peer there is the address `0.x.y.z`, port 1, where `x.y.z` is its id.
  No real datagram comes from `0.0.0.0/8`, and DirectPlay's address objects carry it
  like any other address, so the game hands it back to `Connect`. Everything above the
  socket, connections, retransmission and the game's messages, works on it unchanged.
  The server relays only between a pair it introduced, at most 128 KB/s per player,
  from the source address of the datagram, never from an address the sender claims.
- **After that** the server is out of the game, except as the relay: the transport is
  milestone 1's.

`Direct=0` makes both sides skip the probes and relay everything: the bench's only way
to exercise the relay, since on one machine every probe arrives.

### Showing the join code

The host needs to see the code to pass it on, and the game shows its own
"Local IP Address" as a line in the Internet Game screen's chat box (`chatRoom`), via
`Chat::Append(&chatRoom, "%s", …)`. So the plugin puts its messages in the same place:
"Join code: K7M-Q2X" in GAME SETUP's chat (`chatGame`), where the host waits for
players; "No game with the join code …" or "The online server … does not answer" in
the Internet Game screen's. `Chat::Init` empties a chat box as its screen opens, so a
line is added at once only while that box's window exists (`IsWindow` on the window
`Init` was given), and otherwise waits for that screen's `Init` (the two call sites in
the table above). The host's code is posted again on every `Init` of GAME SETUP until
the session closes, so it is back after a match. A line from the network thread is
added outside our lock, where the game adds its own players' chat lines.

### Running a server

    python3 online/server/a2online-server.py --port 2399     # UDP 2399 open to the internet

It logs one line per game, join and relay to standard output. It keeps everything in
memory, so a restart loses only the codes of games not yet joined. Hosts re-register
within 10 s and get a new code. Limits per source address: 8 hosted games, 64 sockets,
128 KB/s of relay per socket. `selftest.py` beside it runs it on a free port and plays a
host and a joiner against it.

**The public instance** is `c20e.de:2399`, the default `Server=`. It runs there as a
systemd service under a dynamic user, sandboxed, with its memory capped at 128 MB, and
logs to the journal. The bench never uses it: `bench-asi.sh` empties `Server=` unless
told `--server`.

### Not yet established

- **Real routers.** On the bench every game is on one machine, so the probes always
  get through and the relay is only reached with `Direct=0`. Which routers the direct
  path beats, and how often the relay is needed, can only be learnt from players on
  two real networks.

## The plan

1. ~~Does multiplayer work under Proton?~~ No (above).
2. ~~A reference, and the trace.~~ Microsoft's DirectPlay on a bench clone, and
   `Online.asi` logging it (above). Still to trace: leaving, passwords, 3 players,
   and that switching voice on does no harm.
3. ~~Two instances on the bench as a scenario.~~
   `./a2test run multiplayer-two-players` hosts, joins, chats both ways and plays a
   match, unattended, and checks both traces (below). Milestone 1 swaps its `Setup:`
   for our own transport and runs the same steps.
4. ~~The menu entry.~~ *Internet – Online* in the IPX button's place (below);
   `./a2test run multiplayer-online-entry` hosts and joins through it.
5. ~~Milestone 1:~~ our `IDirectPlay8Peer` over plain UDP (above).
   `./a2test run multiplayer-online-match` plays a whole 2-player match on it under plain
   Proton, and `multiplayer-online-loss` the same with 10% of datagrams dropped.
6. ~~Milestone 2:~~ the server, hole punching, relay fallback and join codes (above).
   `./a2test run multiplayer-online-code` joins by code over the direct path,
   `multiplayer-online-relay` through the relay with 5% loss. The public instance runs
   at `c20e.de`. Confirmed in game on one machine, 2026-10-03. Still to do: the first
   games across real routers.
7. **Milestone 3:** the server-backed game list (next section).

**Not planned: voice chat.** DirectPlay Voice is not going to be reimplemented. Players
already have Discord, Steam and the like for that, and they work across every game.

## The game list (milestone 3, planned)

Join codes need the host to pass the code on outside the game. Milestone 3 lets a player
find open games in the game itself: the Internet Game screen's *Current Games* list,
which today shows only the game at a typed address, filled from the server with every
game hosted through it. The aim is the old GameSpy experience: open the screen, see the
games, pick one, join. Codes stay, for games not meant for strangers.

What it involves:

- **Hosts publish a summary.** The host already sends `SV_HOST` every 10 s. It gets the
  game's own summary added: the reply data its game handler gives an
  `ENUM_HOSTS_QUERY` (74 bytes in the trace: game name, players, map). `peer.c` asks its
  own game for it, as a local query, so the summary is always the game's own and
  nothing about its layout has to be known. The server keeps the latest per game and
  drops it with the code, after 40 s of silence or on `SV_BYE`.
- **A new message pair, `SV_LIST` / `SV_GAMES`.** A player who opens the list sends
  `SV_LIST`. The server answers with every listed game: its relay id, code and summary,
  across as many datagrams as it takes (the 1100-byte fragment size applies).
- **Into the game's list.** The player opens the Internet Game screen through *Internet
  – Online* with a blank field. Instead of searching no address, `EnumHosts` then asks the
  server, every 500 ms as the search does now, and each listed game reaches the game as
  an `ENUM_HOSTS_RESPONSE` carrying its summary. *Create Game* stays where it is.
  Because the game ages its list by the 1.5 s enum interval ("Our transport"), a game
  must be re-delivered within that interval or it disappears, so the server's answer is
  cached and re-delivered, not the server asked faster.
- **Joining from the list** is a join by code: picking a game hands `Connect` that
  game's address, which `peer.c` turns into the `SV_JOIN` / punch / relay path of
  milestone 2. Nothing about connecting changes.
- **Ping.** The list has a ping column. A round trip to the server says nothing about
  the host, so either the column shows the server's view or each listed host gets a
  probe; to be decided once the list works.
- **Opting out.** A key in `Online.ini` (a working name: `List=0`) hosts without being
  listed, with a code only.
- **Games in progress.** A game whose match has started cannot be joined. It has to
  leave the list, either because the host stops answering enum queries then (to be
  traced) or because `peer.c` tells the server.
- **Limits.** The server caps how many games it lists and how often a source address
  may ask, as it already caps hosting and the relay.

To be established before building it:

- How the game shows an entry: which fields of the 74-byte reply and of the session
  description it reads for the name, player count and map.
- What the host does with enum queries once the match has started.
- Whether a refit and a stock install can share a game (the game compares CRCs). If not,
  the list should show only games a player can actually join, which means the summary
  needs something to compare.

Tests: `selftest.py` grows a host that lists and a player that reads it back, and a
bench scenario (`multiplayer-online-list`) hosts with the bench server, joins from the
list, and plays a match.

## Running the trace

On the bench, with two clones, using Microsoft's DirectPlay as the reference. Its DLLs are
not redistributable. `reference-dplay.sh` downloads the DirectX redistributable that
`winetricks directplay` uses into `$A2_DATA/reference/directx/` (checked against its
sha256) and installs the DLLs into a **clone's** prefix, never the real one.

Our own transport needs none of this: `online/bench-asi.sh` (a scenario's `Setup:`)
puts only `Online.asi` into a clone, and `multiplayer-online-match` and
`multiplayer-online-loss` run on it. With `--server` it also starts a server on
127.0.0.1:23990 for every game on the machine (the first set-up starts it, and it exits
90 s after the last datagram), and `--relay` sets `Direct=0` as well;
`multiplayer-online-code` and `multiplayer-online-relay` use them. The joiner types the
code it reads from the host's `Online.log` (the step `Type what follows "join code" in
"Online.log" of host`).

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
