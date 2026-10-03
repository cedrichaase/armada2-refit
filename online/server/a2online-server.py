#!/usr/bin/env python3
"""a2online-server -- the rendezvous and relay server for Online.asi.

One UDP port, Python 3.8+ standard library only, no state on disk: run it
anywhere players can reach and name it in Online.ini (`Server=host:port`).

    python3 a2online-server.py [--port 2399] [--bind 0.0.0.0]

What it does, for each game hosted through Internet - Online
(online/README.md, "The server"):

  * gives the host a join code and keeps it while the host keeps registering
    (every 10 s), along with the host's public address -- the one this server
    sees, which is the host's NAT mapping for the game's own socket;
  * looks a code up for a joiner and introduces the two: each learns the
    other's public and LAN address, and both try to reach each other directly
    (hole punching);
  * relays between the two when that fails, and only between pairs it
    introduced, within a bandwidth budget.

The wire format is the client's (peer.c): "A2O", version 1, a type byte, then
little-endian fields; addresses are 4 address bytes and a big-endian port, as
on the wire.  Nothing a client sends is trusted beyond its own source address.
"""

import argparse
import asyncio
import os
import random
import socket
import struct
import sys
import time

MAGIC = b'A2O\x01'
SV_HOST, SV_HOSTED, SV_JOIN, SV_PEER, SV_INTRO, SV_NOTFOUND, SV_RELAY, SV_RELAYED, SV_BYE, SV_ERROR = range(0x40, 0x4a)

ALPHABET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789'   # no I, L, O, 0, 1: read aloud, typed by hand
CODE_LEN = 6

ENDPOINT_IDLE = 60       # s without a datagram before an endpoint (and its relay pairs) is forgotten
GAME_IDLE = 40           # s without a host's registration before its code is free
MAX_GAMES_PER_IP = 8
MAX_ENDPOINTS_PER_IP = 64
RELAY_RATE = 128 * 1024  # bytes/s per endpoint, sustained (a full 8-player game is well under 50 KB/s)
RELAY_BURST = 256 * 1024
MAX_DATAGRAM = 1400


def log(*a):
    print(time.strftime('%Y-%m-%d %H:%M:%S'), *a, flush=True)


def pack_addr(addr):
    return socket.inet_aton(addr[0]) + struct.pack('>H', addr[1])


def unpack_addr(b):
    return socket.inet_ntoa(b[:4]), struct.unpack('>H', b[4:6])[0]


def show_code(code):
    return f'{code[:3]}-{code[3:]}'


class Endpoint:
    """One client socket as this server sees it: (public ip, port)."""

    def __init__(self, eid, addr, now):
        self.id = eid
        self.addr = addr
        self.seen = now
        self.peers = set()          # ids it may relay to
        self.tokens = RELAY_BURST
        self.refill = now
        self.relayed = 0

    def spend(self, n, now):
        self.tokens = min(RELAY_BURST, self.tokens + (now - self.refill) * RELAY_RATE)
        self.refill = now
        if self.tokens < n:
            return False
        self.tokens -= n
        return True


class Game:
    def __init__(self, code, host, token, lan, now):
        self.code = code
        self.host = host            # Endpoint
        self.token = token
        self.lan = lan              # 6 bytes, the host's own idea of its address
        self.seen = now
        self.joins = 0


class Server(asyncio.DatagramProtocol):
    def __init__(self, idle_exit=0):
        self.by_addr = {}
        self.by_id = {}
        self.games = {}
        self.next_id = random.randrange(1, 0x7fffff)
        self.idle_exit = idle_exit
        self.last_packet = time.monotonic()
        self.transport = None

    # -- plumbing

    def connection_made(self, transport):
        self.transport = transport

    def send(self, addr, kind, body=b''):
        self.transport.sendto(MAGIC + bytes([kind]) + body, addr)

    def endpoint(self, addr, now):
        ep = self.by_addr.get(addr)
        if ep:
            ep.seen = now
            return ep
        if sum(1 for a in self.by_addr if a[0] == addr[0]) >= MAX_ENDPOINTS_PER_IP:
            return None
        while True:
            self.next_id = self.next_id % 0xfffffe + 1          # 24 bits: the client's 0.x.y.z
            if self.next_id not in self.by_id:
                break
        ep = Endpoint(self.next_id, addr, now)
        self.by_addr[addr] = ep
        self.by_id[ep.id] = ep
        return ep

    def new_code(self):
        while True:
            code = ''.join(random.SystemRandom().choice(ALPHABET) for _ in range(CODE_LEN))
            if code not in self.games:
                return code

    # -- datagrams

    def datagram_received(self, data, addr):
        now = time.monotonic()
        self.last_packet = now
        if len(data) < 5 or data[:4] != MAGIC or len(data) > MAX_DATAGRAM:
            return
        kind, body = data[4], data[5:]
        ep = self.endpoint(addr, now)
        if not ep:
            return
        try:
            if kind == SV_HOST:
                self.on_host(ep, body, now)
            elif kind == SV_JOIN:
                self.on_join(ep, body, now)
            elif kind == SV_RELAY:
                self.on_relay(ep, body, now)
            elif kind == SV_BYE:
                self.on_bye(ep, body)
        except (struct.error, IndexError, ValueError):
            pass

    def on_host(self, ep, b, now):
        """[token u32][lan addr 6]: register, or refresh, the game this endpoint hosts.
        The token keeps the code when the host's NAT gives it a new port."""
        token = struct.unpack_from('<I', b)[0]
        lan = b[4:10]
        game = next((g for g in self.games.values() if g.token == token and
                     g.host.addr[0] == ep.addr[0]), None)
        if game:
            if game.host is not ep:
                log(f'game {show_code(game.code)}: host moved {game.host.addr} -> {ep.addr}')
                game.host = ep
        else:
            if sum(1 for g in self.games.values() if g.host.addr[0] == ep.addr[0]) >= MAX_GAMES_PER_IP:
                self.error(ep, token, 'too many games hosted from this address')
                return
            game = Game(self.new_code(), ep, token, lan, now)
            self.games[game.code] = game
            log(f'game {show_code(game.code)}: hosted by {ep.addr} (lan {unpack_addr(lan)}, id {ep.id})')
        game.seen = now
        game.lan = lan
        self.send(ep.addr, SV_HOSTED, struct.pack('<I', token) + game.code.encode() +
                  pack_addr(ep.addr) + struct.pack('<I', ep.id))

    def on_join(self, ep, b, now):
        """[nonce u32][code 6][lan addr 6]: introduce a joiner and the host."""
        nonce = struct.unpack_from('<I', b)[0]
        code = b[4:10].decode('ascii', 'replace').upper()
        lan = b[10:16]
        game = self.games.get(code)
        if not game or game.host is ep:
            self.send(ep.addr, SV_NOTFOUND, struct.pack('<I', nonce))
            if not game:
                log(f'join {show_code(code)} from {ep.addr}: no such game')
            return
        host = game.host
        pair = random.getrandbits(32) or 1
        ep.peers.add(host.id)
        host.peers.add(ep.id)
        game.joins += 1
        self.send(ep.addr, SV_PEER, struct.pack('<I', nonce) + pack_addr(host.addr) + game.lan +
                  struct.pack('<III', host.id, ep.id, pair))
        self.send(host.addr, SV_INTRO, pack_addr(ep.addr) + lan + struct.pack('<II', ep.id, pair))
        log(f'join {show_code(code)}: {ep.addr} (lan {unpack_addr(lan)}, id {ep.id}) -> host {host.addr}')

    def on_relay(self, ep, b, now):
        """[to id u32][datagram]: forward to a peer this server introduced."""
        to = struct.unpack_from('<I', b)[0]
        dst = self.by_id.get(to)
        if not dst or to not in ep.peers or not ep.spend(len(b), now):
            return
        if not ep.relayed:
            log(f'relay: {ep.addr} (id {ep.id}) -> {dst.addr} (id {dst.id})')
        ep.relayed += len(b) - 4
        self.transport.sendto(MAGIC + bytes([SV_RELAYED]) + struct.pack('<I', ep.id) + b[4:], dst.addr)

    def on_bye(self, ep, b):
        token = struct.unpack_from('<I', b)[0]
        for code, g in list(self.games.items()):
            if g.host is ep and g.token == token:
                del self.games[code]
                log(f'game {show_code(code)}: closed by its host ({g.joins} joins)')

    def error(self, ep, ref, text):
        self.send(ep.addr, SV_ERROR, struct.pack('<I', ref) + text.encode()[:200])
        log(f'refused {ep.addr}: {text}')

    # -- housekeeping

    def sweep(self):
        now = time.monotonic()
        for code, g in list(self.games.items()):
            if now - g.seen > GAME_IDLE:
                del self.games[code]
                log(f'game {show_code(code)}: host silent, code freed ({g.joins} joins)')
        for addr, ep in list(self.by_addr.items()):
            if now - ep.seen > ENDPOINT_IDLE:
                del self.by_addr[addr]
                del self.by_id[ep.id]
                for pid in ep.peers:
                    other = self.by_id.get(pid)
                    if other:
                        other.peers.discard(ep.id)
                if ep.relayed:
                    log(f'relay: {addr} (id {ep.id}) gone after {ep.relayed} bytes')
        return self.idle_exit and now - self.last_packet > self.idle_exit


async def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--port', type=int, default=2399)
    ap.add_argument('--bind', default='0.0.0.0')
    ap.add_argument('--idle-exit', type=int, default=0, metavar='S',
                    help='exit after S seconds without a datagram (the test bench uses this)')
    args = ap.parse_args()
    loop = asyncio.get_running_loop()
    try:
        transport, server = await loop.create_datagram_endpoint(
            lambda: Server(args.idle_exit), local_addr=(args.bind, args.port))
    except OSError as e:
        log(f'cannot listen on UDP {args.bind}:{args.port}: {e}')
        return 2
    log(f'a2online-server listening on UDP {args.bind}:{args.port} (pid {os.getpid()})')
    try:
        while True:
            await asyncio.sleep(5)
            if server.sweep():
                log(f'no datagram for {args.idle_exit} s: exiting')
                return 0
    finally:
        transport.close()


if __name__ == '__main__':
    try:
        sys.exit(asyncio.run(main()))
    except KeyboardInterrupt:
        pass
