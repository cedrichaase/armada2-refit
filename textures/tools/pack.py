#!/usr/bin/env python3
"""Texture packs: the built textures of some targets, in one zip, to install elsewhere.

    pack.py make OUT.zip NAME DATA COMMIT TARGET...   zip DATA/<T>/out/*.tga + pack.txt
    pack.py open PACK.zip DEST                        check every hash, unpack to DEST
    pack.py list PACK.zip                             print pack.txt

Layout inside the zip -- the same as a target's work directory in A2_DATA, so that
`a2tex install --pack` can point DATA at the unpacked copy and run the ordinary install,
every guard included (interface-sprite ceiling, mip chains, install=no, panel=):

    pack.txt                      name, date, commit, targets, then "<sha256>  <path>"
    textures/<T>/out/<name>.tga

A pack holds only BUILT textures: no stock art, no ai/ or src/ layers. It is still the
game's art, upscaled -- whoever shares one is responsible for what they share
(textures/PACKS.md). Nothing here writes outside OUT or DEST.
"""
import datetime, hashlib, os, re, sys, zipfile

MANIFEST = 'pack.txt'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def make(out, name, data, commit, targets):
    rows, files = [], []
    for t in targets:
        d = os.path.join(data, t, 'out')
        tgas = sorted(f for f in os.listdir(d) if f.lower().endswith('.tga')) if os.path.isdir(d) else []
        if not tgas:
            sys.exit(f'{t}: nothing built in {d}')
        for f in tgas:
            files.append((os.path.join(d, f), f'textures/{t}/out/{f}'))
    tmp = out + '.tmp'
    with zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED) as z:
        for src, arc in files:
            b = open(src, 'rb').read()
            rows.append(f'{sha(b)}  {arc}')
            z.writestr(arc, b)
        head = [f'name: {name}',
                f'made: {datetime.date.today().isoformat()}',
                f'commit: {commit}',
                f'targets: {" ".join(targets)}',
                '']
        z.writestr(MANIFEST, '\n'.join(head + rows) + '\n')
    os.replace(tmp, out)
    print(f'pack  {out}  {len(targets)} target(s), {len(files)} file(s), '
          f'{os.path.getsize(out) // (1 << 20)} MB')


def read_manifest(z):
    try:
        text = z.read(MANIFEST).decode()
    except KeyError:
        sys.exit(f'no {MANIFEST} in the zip -- not a texture pack')
    head, hashes = {}, {}
    for line in text.splitlines():
        m = re.match(r'^([0-9a-f]{64})  (.+)$', line)
        if m:
            hashes[m.group(2)] = m.group(1)
        elif ': ' in line:
            k, v = line.split(': ', 1)
            head[k] = v
    return head, hashes


def open_pack(pack, dest):
    with zipfile.ZipFile(pack) as z:
        head, hashes = read_manifest(z)
        names = [n for n in z.namelist() if n != MANIFEST and not n.endswith('/')]
        bad = []
        for n in names:
            # Only textures/<T>/out/<file>.tga, nothing that climbs out of DEST.
            parts = n.split('/')
            if (len(parts) != 4 or parts[0] != 'textures' or parts[2] != 'out'
                    or not parts[3].lower().endswith('.tga') or '..' in parts
                    or parts[1] in ('', '.') or parts[3].startswith('.')):
                bad.append(f'unexpected path {n}')
                continue
            if n not in hashes:
                bad.append(f'{n} is not in {MANIFEST}')
            elif sha(z.read(n)) != hashes[n]:
                bad.append(f'{n} does not match its hash')
        missing = sorted(set(hashes) - set(names))
        bad += [f'{p} is in {MANIFEST} but not in the zip' for p in missing]
        if bad:
            sys.exit('refusing the pack:\n  ' + '\n  '.join(bad))
        for n in names:
            dst = os.path.join(dest, *n.split('/'))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            with open(dst, 'wb') as f:
                f.write(z.read(n))
    print(f"pack  {head.get('name', '?')} (made {head.get('made', '?')} at "
          f"{head.get('commit', '?')}): {len(names)} file(s), every hash checked")


def main(argv):
    if len(argv) >= 6 and argv[1] == 'make':
        make(argv[2], argv[3], argv[4], argv[5], argv[6:])
    elif len(argv) == 4 and argv[1] == 'open':
        open_pack(argv[2], argv[3])
    elif len(argv) == 3 and argv[1] == 'list':
        with zipfile.ZipFile(argv[2]) as z:
            sys.stdout.write(z.read(MANIFEST).decode())
    else:
        sys.exit(__doc__)


if __name__ == '__main__':
    main(sys.argv)
