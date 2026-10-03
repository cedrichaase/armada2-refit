#!/usr/bin/env python3
"""Runs a2online-server.py on a free local port and plays a host and a joiner
against it with plain sockets: register, look the code up, the introduction,
a relayed datagram each way, a relay to someone never introduced (dropped), an
unknown code, and the code freed by BYE.  Exit 0 when all of it holds.

    python3 online/server/selftest.py
"""

import socket
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
MAGIC = b'A2O\x01'
SV_HOST, SV_HOSTED, SV_JOIN, SV_PEER, SV_INTRO, SV_NOTFOUND, SV_RELAY, SV_RELAYED, SV_BYE, SV_ERROR = range(0x40, 0x4a)
LAN = socket.inet_aton('192.168.1.20') + struct.pack('>H', 2302)

failed = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        failed.append(what)


def sock():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.bind(('127.0.0.1', 0))
    s.settimeout(2)
    return s


def recv(s, kind):
    try:
        while True:
            d, _ = s.recvfrom(2048)
            if d[:4] == MAGIC and d[4] == kind:
                return d[5:]
    except socket.timeout:
        return None


def main():
    probe = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    probe.bind(('127.0.0.1', 0))
    port = probe.getsockname()[1]
    probe.close()
    srv = subprocess.Popen([sys.executable, str(HERE / 'a2online-server.py'), '--bind', '127.0.0.1',
                            '--port', str(port)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    try:
        time.sleep(0.5)
        S = ('127.0.0.1', port)
        host, joiner, stranger = sock(), sock(), sock()

        host.sendto(MAGIC + bytes([SV_HOST]) + struct.pack('<I', 0x1234) + LAN, S)
        r = recv(host, SV_HOSTED)
        check(r is not None and struct.unpack_from('<I', r)[0] == 0x1234, 'host registers and gets an answer')
        code = r[4:10].decode()
        pub = (socket.inet_ntoa(r[10:14]), struct.unpack('>H', r[14:16])[0])
        host_id = struct.unpack_from('<I', r, 16)[0]
        check(len(code) == 6 and pub == host.getsockname(), f'code {code}, public address {pub} is the socket')

        host.sendto(MAGIC + bytes([SV_HOST]) + struct.pack('<I', 0x1234) + LAN, S)
        r = recv(host, SV_HOSTED)
        check(r is not None and r[4:10].decode() == code, 'registering again keeps the code')

        joiner.sendto(MAGIC + bytes([SV_JOIN]) + struct.pack('<I', 77) + code.lower().encode() + LAN, S)
        p = recv(joiner, SV_PEER)
        i = recv(host, SV_INTRO)
        check(p is not None and struct.unpack_from('<I', p)[0] == 77, 'joiner gets the host (code in any case)')
        check(i is not None, 'host is told about the joiner')
        if p is None or i is None:
            return
        hpub = (socket.inet_ntoa(p[4:8]), struct.unpack('>H', p[8:10])[0])
        hid, jid, pair = struct.unpack_from('<III', p, 16)
        check(hpub == host.getsockname() and hid == host_id and p[10:16] == LAN, "joiner has the host's addresses")
        jpub = (socket.inet_ntoa(i[0:4]), struct.unpack('>H', i[4:6])[0])
        check(jpub == joiner.getsockname() and struct.unpack_from('<II', i, 12) == (jid, pair),
              "host has the joiner's address, id and the same pair token")

        joiner.sendto(MAGIC + bytes([SV_RELAY]) + struct.pack('<I', hid) + b'A2O\x01\x05hello', S)
        r = recv(host, SV_RELAYED)
        check(r is not None and struct.unpack_from('<I', r)[0] == jid and r[4:] == b'A2O\x01\x05hello',
              'joiner -> host through the relay')
        host.sendto(MAGIC + bytes([SV_RELAY]) + struct.pack('<I', jid) + b'A2O\x01\x05back', S)
        r = recv(joiner, SV_RELAYED)
        check(r is not None and struct.unpack_from('<I', r)[0] == hid and r[4:] == b'A2O\x01\x05back',
              'host -> joiner through the relay')

        stranger.sendto(MAGIC + bytes([SV_RELAY]) + struct.pack('<I', hid) + b'x', S)
        host.settimeout(0.5)
        check(recv(host, SV_RELAYED) is None, 'a relay from someone never introduced is dropped')
        host.settimeout(2)

        stranger.sendto(MAGIC + bytes([SV_JOIN]) + struct.pack('<I', 5) + b'ZZZZZZ' + LAN, S)
        check(recv(stranger, SV_NOTFOUND) is not None, 'an unknown code is answered NOTFOUND')

        host.sendto(MAGIC + bytes([SV_BYE]) + struct.pack('<I', 0x1234), S)
        time.sleep(0.2)
        stranger.sendto(MAGIC + bytes([SV_JOIN]) + struct.pack('<I', 6) + code.encode() + LAN, S)
        check(recv(stranger, SV_NOTFOUND) is not None, 'BYE frees the code')
    finally:
        srv.terminate()
        out = srv.communicate(timeout=5)[0]
        print('--- server log\n' + out.rstrip())
    print('\n' + ('all passed' if not failed else f'{len(failed)} FAILED'))
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
