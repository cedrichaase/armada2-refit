#!/usr/bin/env python3
"""The equivalence test of AR-3 (PLAN-ARCHITECTURE.md): every struct offset and value
moved out of a plugin's source into its profile resolves, on the build the source was
written for, to the value the source had.

    moved.py            check moved-gog-1.1.tsv against profiles/gog-1.1; exit 1 on a difference

`moved-<build>.tsv` lists, one per line, the profile file, the entry's id, the value the
source had (hex, or a negative decimal) and the source file it was in. The list was
written from the sources before they changed; the profile is what the plugins now read,
so a difference here is an offset that moved by mistake (a crash, not a stand-down).
Each entry must also be a `field` or a `value`, used by a hook of its file, and appear
once in the list. Stdlib only.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import profiles  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))


def check(build='gog-1.1', game='armada2'):
    path = os.path.join(HERE, f'moved-{build}.tsv')
    games = profiles.load_game_sets()
    gs = games[game]
    b = next(x for x in gs.builds if x.name == build)
    flats = {}
    bad, seen, n = [], set(), 0
    for ln, raw in enumerate(open(path), 1):
        line = raw.rstrip('\n')
        if not line or line.startswith('#'):
            continue
        parts = line.split('\t')
        if len(parts) != 4:
            bad.append(f'{path}:{ln}: four tab-separated columns wanted')
            continue
        fname, sid, want, src = parts
        n += 1
        if (fname, sid) in seen:
            bad.append(f'{path}:{ln}: {fname} {sid} listed twice')
        seen.add((fname, sid))
        if fname not in gs.files:
            bad.append(f'{path}:{ln}: no profile file {fname}')
            continue
        if fname not in flats:
            flats[fname] = profiles.flat(gs, fname, b)
        e = flats[fname].get(sid)
        if e is None:
            bad.append(f'{path}:{ln}: {fname} has no {sid} in {build} (was {want} in {src})')
            continue
        kind = e.get('kind')
        if kind not in ('field', 'value'):
            bad.append(f'{path}:{ln}: {sid} is a {kind}, not a field or value')
            continue
        got = e.get('offset' if kind == 'field' else 'value')
        if got != int(want, 0):
            bad.append(f'{path}:{ln}: {fname} {sid} resolves to {got:#x} on {build}; {src} had {want}')
        if not e.get('used_by'):
            bad.append(f'{path}:{ln}: {sid} is used by no hook')
    return n, bad


def main(argv):
    if argv:
        print(__doc__, file=sys.stderr)
        return 2
    n, bad = check()
    for b in bad:
        print(f'moved.py: {b}')
    print(f'moved.py: {n} moved offsets and values, {len(bad)} differences')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
