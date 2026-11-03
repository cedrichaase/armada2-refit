#!/usr/bin/env python3
"""Compares the profiles' names with the game's own symbol map (developer machines only).

    annotate.py [--map PATH] [--profiles DIR ...]

Reads `$A2_GAME/armada2.map` (a2env.py says where; the map is never in the repository),
and prints each entry of the root build of every profile set whose `in` (call, jmp, label,
patch, scan) or `symbol` (fn, vtable, global, rtti) disagrees with the map: the function
that contains its address, or the symbol that starts at it. Report only: it writes nothing,
prints names and addresses only (hard rule 9), and exits 0. `slot`, `field` and `value`
entries have no address of their own and are skipped. The map describes the stock build,
so only each game's root build is compared.

Names are demangled simply: `?name@Class@Ns@@...` -> `Ns::Class::name`, `?name@@Y...` ->
`name`, constructors and destructors, `_name@12` -> `name`. A vtable or rti symbol agrees
with its class name (with or without `::vftable`).
"""
import bisect
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import profiles  # noqa: E402

ROOT = profiles.ROOT
sys.path.insert(0, ROOT)
LINE = re.compile(r'^\s*([0-9a-fA-F]{4}):[0-9a-fA-F]{8}\s+(\S+)\s+([0-9a-fA-F]{8})\b')
OPS = {'4': 'operator=', '8': 'operator==', '9': 'operator!=', 'A': 'operator[]', 'C': 'operator->',
       'D': 'operator*', 'E': 'operator++', 'F': 'operator--', 'G': 'operator-', 'H': 'operator+',
       'I': 'operator&', 'K': 'operator/', 'M': 'operator<', 'O': 'operator>', 'P': 'operator>=',
       'R': 'operator()', 'S': 'operator!', 'U': 'operator|', '2': 'operator new', '3': 'operator delete'}


def demangle(n):
    """A short, best-effort name for a decorated one."""
    if n.startswith('??_7') or n.startswith('??_R4') or n.startswith('??_R0'):
        pre = 4 if n.startswith('??_7') else 5
        body = n[pre:]
        if body.startswith('?AV') or body.startswith('?AU'):
            body = body[3:]
        parts = body.split('@@')[0].split('@')
        return '::'.join(reversed([p for p in parts if p]))
    if n.startswith('?'):
        body = n[1:]
        special = None
        if body.startswith('?'):
            code = body[1:2]
            if code == '_':
                special, body = 'scalar', body[3:]
            else:
                special, body = code, body[2:]
        parts = []
        for p in body.split('@'):
            if p == '':
                break
            parts.append(p)
        parts.reverse()                           # Ns, Class, name
        if special == '0' and parts:
            return '::'.join(parts + [parts[-1]])
        if special in ('1', 'scalar') and parts:
            return '::'.join(parts + ['~' + parts[-1]])
        if special in OPS:
            return '::'.join(parts + [OPS[special]])
        return '::'.join(parts)
    m = re.match(r'^[_@]?([^@]+?)(?:@\d+)?$', n)
    return m.group(1) if m else n


def load_map(path):
    syms = []
    on = False
    with open(path, encoding='utf-8', errors='replace') as f:
        for ln in f:
            if 'Publics by Value' in ln:
                on = True
                continue
            if not on:
                continue
            m = LINE.match(ln)
            if m:
                syms.append((int(m.group(3), 16), m.group(1) == '0001', demangle(m.group(2))))
    syms.sort(key=lambda s: s[0])
    return syms


def main(argv):
    mp, roots = None, None
    a = list(argv)
    while a:
        x = a.pop(0)
        if x == '--map' and a:
            mp = a.pop(0)
        elif x == '--profiles':
            roots = []
            while a and not a[0].startswith('--'):
                roots.append(os.path.abspath(a.pop(0)))
        else:
            print(__doc__, file=sys.stderr)
            return 2
    if mp is None:
        import a2env
        mp = os.path.join(a2env.GAME, 'armada2.map')
    if not os.path.isfile(mp):
        print(f'annotate.py: no map at {mp} (set A2_GAME or pass --map)', file=sys.stderr)
        return 2
    syms = load_map(mp)
    if not syms:
        print('annotate.py: the map has no "Publics by Value" lines', file=sys.stderr)
        return 2
    code = [s for s in syms if s[1]]
    vas = [s[0] for s in syms]
    cvas = [s[0] for s in code]

    def enclosing(addr):
        i = bisect.bisect_right(cvas, addr) - 1
        return (code[i][2], cvas[i]) if i >= 0 else (None, 0)

    def exact(addr):
        i = bisect.bisect_left(vas, addr)
        out = []
        while i < len(vas) and vas[i] == addr:
            out.append(syms[i][2])
            i += 1
        return out

    ok = bad = skipped = 0
    games = profiles.load_game_sets(roots)
    for gs in games.values():
        b = gs.root
        for fname, rf in gs.files.items():
            for s in profiles.flat(gs, fname, b).values():
                kind, sid = s.get('kind'), s.get('id')
                addr = s.get('addr')
                where = f'{rf.rel()}: {sid} ({kind}) at {addr:#x}' if isinstance(addr, int) else None
                if kind in ('call', 'jmp', 'label', 'patch', 'scan') and isinstance(addr, int) and s.get('in'):
                    fn, start = enclosing(addr)
                    if fn == s['in']:
                        ok += 1
                    else:
                        bad += 1
                        print(f'{where}: in says {s["in"]}, the map has {fn} (from {start:#x})')
                elif kind in ('fn', 'vtable', 'global', 'rtti') and isinstance(addr, int):
                    names = exact(addr)
                    want = s.get('symbol', '')
                    if kind in ('vtable', 'rtti'):
                        hit = any(n == want or n.split('::vftable')[0] == want for n in names)
                    else:
                        hit = want in names
                    if hit:
                        ok += 1
                    else:
                        bad += 1
                        if names:
                            print(f'{where}: symbol says {want}, the map has {" / ".join(names)} there')
                        else:
                            fn, start = enclosing(addr)
                            print(f'{where}: symbol says {want}, no map symbol starts there (inside {fn} from {start:#x})')
                else:
                    skipped += 1
    print(f'ok: {ok} entries agree' + (f'; {bad} disagree' if bad else '') + f' ({skipped} without an address to compare)')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
