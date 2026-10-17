# Online play, scouted
Film: online-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

A scout for the promo's online beat: two games on the bench, side by side, through
*Internet - Online* on a local `a2online-server` (the bench never talks to the public
one). The promo uses about 6 to 10 seconds of o1 and, if it reaches a match, o2.

## Online

### o1 · Create, type the code, join
Shows: online: Internet - Online, join code, no port forwarding
Players: host, joiner
Prepare: online/bench-asi.sh --server
Args: -nointro
Scene: menu.ini
Layout: split 0.75
In: 0
Length: 10
Status: draft

```setup
step Wait for the main menu
step Click the multiplayer emblem
step Wait for the multiplayer connection screen
step Click at design 400,187
step Press End
step Press BackSpace 32 times
host: step Type "host"
joiner: step Type "joiner"
host: step Click "Internet - Online"
host: step Wait up to 10 seconds for "Join code or host address"
host: step Click "O.K."
host: step Wait up to 30 seconds for "Create Game"
host: step Click "Create Game"
host: wait 3
host: step Click "O.K."
host: step Wait up to 30 seconds for "GAME SETUP"
host: step Wait up to 20 seconds for "Join code"
```
```action
wait 1
joiner: step Click "Internet - Online"
joiner: step Wait up to 10 seconds for "Join code or host address"
joiner: step Click at design 400,292
joiner: typelog host Online.log "join code"
wait 0.6
joiner: step Click "O.K."
joiner: step Wait up to 30 seconds for "host's Game"
wait 0.8
joiner: step Click "host's Game"
joiner: step Click "Join Game"
joiner: step Wait up to 30 seconds for "GAME SETUP"
wait 4
```

### o2 · Both games launch into the same match
Shows: online: a match on our own transport
Players: host, joiner
Prepare: online/bench-asi.sh --server
Args: -nointro
Scene: menu.ini
Layout: split
In: 0
Length: 10
Status: draft

```setup
step Wait for the main menu
step Click the multiplayer emblem
step Wait for the multiplayer connection screen
step Click at design 400,187
step Press End
step Press BackSpace 32 times
host: step Type "host"
joiner: step Type "joiner"
host: step Click "Internet - Online"
host: step Wait up to 10 seconds for "Join code or host address"
host: step Click "O.K."
host: step Wait up to 30 seconds for "Create Game"
host: step Click "Create Game"
host: wait 3
host: step Click "O.K."
host: step Wait up to 30 seconds for "GAME SETUP"
host: step Wait up to 20 seconds for "Join code"
joiner: step Click "Internet - Online"
joiner: step Wait up to 10 seconds for "Join code or host address"
joiner: step Click at design 400,292
joiner: typelog host Online.log "join code"
joiner: step Click "O.K."
joiner: step Wait up to 30 seconds for "host's Game"
joiner: step Click "host's Game"
joiner: step Click "Join Game"
joiner: step Wait up to 30 seconds for "GAME SETUP"
```
```action
host: step Click "LAUNCH"
wait 5
joiner: step Click "LAUNCH"
wait 5
host: step Click "LAUNCH"
wait 22
```
