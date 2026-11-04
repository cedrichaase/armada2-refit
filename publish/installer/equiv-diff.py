#!/usr/bin/env python3
"""What two game directories differ in, for the installer equivalence test (equiv.sh).

  equiv-diff.py --ini-keys A.ini B.ini
      "same" when both hold the same keys with the same values in the same order
      (sections and keys compared in lower case, values trimmed, comments and blank
      lines ignored), else what differs.
  equiv-diff.py --trees --pair P --scenario S [--known FILE] [--used FILE] NAME_A DIR_A NAME_B DIR_B
      every path under either directory: present in both, the same kind (file or
      folder), and every file byte for byte. An .ini that differs is also compared key
      by key, so the report says what a player would see. A difference whose path
      matches a line of FILE for this pair and scenario is printed as known, with its
      reason; the matched lines are appended to --used. Exit 1 on any other difference.

The known file: one difference a line, `<pair> <scenario glob> <path glob> <how> <reason>`,
'#' comments. Pairs: sh-ps (install.sh and install.ps1), sh-repo (install.sh and the
repository's ./install). <how> is `any` (the path may differ in any way) or `comments`
(the files may differ only in their comment lines, '#' or ';': anything else still fails).
"""
import fnmatch
import hashlib
import os
import re
import sys

SEC = re.compile(r'^\s*\[([^\]]+)\]')
KEY = re.compile(r'^\s*(\w+)\s*=\s*(.*?)\s*$')


def ini_keys(path):
    out, cur = [], None
    with open(path, 'rb') as f:
        text = f.read().decode('latin-1')
    for line in text.splitlines():
        m = SEC.match(line)
        if m:
            cur = m.group(1).strip().lower()
            continue
        if line.lstrip().startswith((';', '#')):
            continue
        m = KEY.match(line)
        if m:
            out.append((cur, m.group(1).lower(), m.group(2)))
    return out


def ini_diff(a, b, na='a', nb='b'):
    """Lines saying how two ini files differ key by key; [] when they do not."""
    ka, kb = ini_keys(a), ini_keys(b)
    if ka == kb:
        return []
    da = {(s, k): v for s, k, v in ka}
    db = {(s, k): v for s, k, v in kb}
    out = []
    for sk in sorted(set(da) | set(db), key=lambda x: (str(x[0]), x[1])):
        name = f'[{sk[0]}] {sk[1]}' if sk[0] is not None else f'(no section) {sk[1]}'
        if sk not in db:
            out.append(f'{name}: {na}={da[sk]!r}, {nb} has no such key')
        elif sk not in da:
            out.append(f'{name}: {na} has no such key, {nb}={db[sk]!r}')
        elif da[sk] != db[sk]:
            out.append(f'{name}: {na}={da[sk]!r}, {nb}={db[sk]!r}')
    if not out:
        if len(ka) != len(kb):
            out.append(f'a key is there twice in one: {na} {len(ka)} key lines, {nb} {len(kb)}')
        else:
            out.append('the same keys and values, in another order')
    return out


def uncommented(path):
    with open(path, 'rb') as f:
        return [l for l in f.read().decode('latin-1').splitlines()
                if not l.lstrip().startswith(('#', ';'))]


def walk(d):
    out = {}
    for base, dirs, files in os.walk(d):
        rel = os.path.relpath(base, d)
        for n in dirs:
            out[os.path.normpath(os.path.join(rel, n))] = 'dir'
        for n in files:
            p = os.path.join(base, n)
            if os.path.islink(p):
                out[os.path.normpath(os.path.join(rel, n))] = 'link ' + os.readlink(p)
            else:
                with open(p, 'rb') as f:
                    out[os.path.normpath(os.path.join(rel, n))] = hashlib.sha256(f.read()).hexdigest()
    return out


def load_known(path, pair, scenario):
    known = []
    if not path:
        return known
    with open(path) as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith('#'):
                continue
            p, sc, glob, how, reason = (line.split(None, 4) + [''] * 5)[:5]
            if how not in ('any', 'comments'):
                sys.exit(f'{path}: <how> is any or comments, not {how!r}: {line}')
            if p == pair and fnmatch.fnmatchcase(scenario, sc):
                known.append((glob, how, reason, line))
    return known


def main(argv):
    if argv[:1] == ['--ini-keys']:
        d = ini_diff(argv[1], argv[2])
        print('same' if not d else '\n'.join(d))
        return 0
    args, opts = [], {}
    it = iter(argv[1:] if argv[:1] == ['--trees'] else argv)
    for a in it:
        if a in ('--pair', '--scenario', '--known', '--used'):
            opts[a[2:]] = next(it)
        else:
            args.append(a)
    na, da, nb, db = args
    pair, scenario = opts.get('pair', ''), opts.get('scenario', '')
    known = load_known(opts.get('known'), pair, scenario)
    ta, tb = walk(da), walk(db)
    bad = 0
    used = []
    for p in sorted(set(ta) | set(tb)):
        a, b = ta.get(p), tb.get(p)
        if a == b:
            continue
        if a is None:
            what = [f'only after {nb}']
        elif b is None:
            what = [f'only after {na}']
        elif 'dir' in (a, b):
            what = [f'a folder after one, a file after the other']
        else:
            what = ['the bytes differ']
            if p.lower().endswith('.ini'):
                d = ini_diff(os.path.join(da, p), os.path.join(db, p), na, nb)
                what = ['the same keys and values; the bytes differ (line ends, comments, layout)'] if not d else d
        hit = next((k for k in known if fnmatch.fnmatchcase(p, k[0])), None)
        if hit and hit[1] == 'comments' and (a is None or b is None or 'dir' in (a, b)
                                             or uncommented(os.path.join(da, p)) != uncommented(os.path.join(db, p))):
            what = ['differs in more than its comments'] + what
            hit = None
        if hit:
            print(f'known {p}: {what[0]} -- {hit[2]}')
            used.append(hit[3])
        else:
            print(f'FAIL  {p}: {what[0]}')
            for w in what[1:]:
                print(f'        {w}')
            bad += 1
    if opts.get('used') and used:
        with open(opts['used'], 'a') as f:
            f.write(''.join(u + '\n' for u in used))
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
