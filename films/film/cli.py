"""./a2film -- the commands (films/README.md)."""
import argparse
import json
import os
import subprocess
import sys
import time

from . import code
from . import storyboard as sb

COMMANDS = {}


def command(fn=None, shots=True, options=()):
    """Register a subcommand: its docstring is its help, `options` its extra arguments."""
    def reg(f):
        f.shots, f.options = shots, list(options)
        COMMANDS[f.__name__] = f
        return f
    return reg(fn) if fn else reg


def load(path):
    try:
        return sb.parse(path)
    except sb.BoardError as e:
        sys.exit(f'a2film: {e}')


def select(film, ids):
    if not ids:
        return film.shots
    try:
        return [film.shot(i) for i in ids]
    except sb.BoardError as e:
        sys.exit(f'a2film: {e}')


def total_length(film):
    return sum(s.length or 0.0 for s in film.shots)


def take_check(film, shot, variant, tier, ignore_code=False):
    """A take's state and why: ('-', None) no take; ('ok', None) current; ('spec', why) filmed
    from a spec that has since changed; ('code', why) the code it shows has changed since it
    was filmed; ('unknown', why) it does not say which code it shows (films/README.md,
    "When a take is current"). With `ignore_code` only the spec counts."""
    stem = film.take(shot, variant, tier)
    if not os.path.exists(stem + '.mkv'):
        return '-', None
    try:
        spec = json.load(open(stem + '.spec.json'))
    except (OSError, ValueError):
        spec = {}
    if spec.get('hash') != film.spec_hash(shot, variant, tier):
        return 'spec', 'spec changed'
    if ignore_code:
        return 'ok', None
    try:
        return code.check(spec, film, shot, variant)
    except RuntimeError as e:
        return 'unknown', str(e)


TIER_ORDER = ('final', 'probe')


def best_take(film, shot, variant, tier, ignore_code=False):
    """The take a cut at `tier` uses for this shot and variant, as (stem, tier) or (None, None).
    A final cut uses final takes only. A probe cut uses the best there is of either tier:
    current before stale, a stale one whose spec still holds before one whose spec changed,
    then the larger frame (a final take, or a probe at the shot's own `Tier:`). With one
    take only, that one, unchecked."""
    tiers = ('final',) if tier == 'final' else TIER_ORDER
    have = [t for t in tiers if os.path.exists(film.take(shot, variant, t) + '.mkv')]
    if not have:
        return None, None
    if len(have) == 1:
        return film.take(shot, variant, have[0]), have[0]

    def rank(t):
        st, _ = take_check(film, shot, variant, t, ignore_code)
        try:
            res = json.load(open(film.take(shot, variant, t) + '.spec.json')).get('res')
        except (OSError, ValueError):
            res = None
        w, h = (int(x) for x in (res or film.res(t, shot)).lower().split('x'))
        return (st == 'ok', st != 'spec', w * h)
    t = max(have, key=rank)
    return film.take(shot, variant, t), t


def take_state(film, shot, variant, tier, ignore_code=False):
    """'-' no take, 'ok' current, else why not: 'spec', 'code' or 'unknown' (take_check)."""
    return take_check(film, shot, variant, tier, ignore_code)[0]


IGNORE_CODE = (('--ignore-code',), dict(action='store_true',
               help='a take whose code changed (or is unknown) counts as current: only a changed spec makes it stale'))


@command(shots=False)
def check(a):
    """parse a storyboard, list its shots and what is wrong with it"""
    film = load(a.board)
    print(f'{film.title} ({film.name}): {len(film.sections)} sections, {len(film.shots)} shots, '
          f'{total_length(film):.1f} s')
    if film.files:
        print(f'  includes {", ".join(film.files)}')
    main_file = os.path.basename(film.path)
    for sec in film.sections:
        print(f'\n  {sec.title}' + (f'   [music: {sec.fields["Music"]}]' if 'Music' in sec.fields else ''))
        for s in sec.shots:
            lay = ' '.join([s.layout[0]] + s.layout[1])
            var = ','.join(s.variants) if s.filmed else ''
            extra = (f'  [tier {s.tier}]' if s.filmed and s.get('Tier') else '') + \
                    (f'  [{s.file}]' if s.file != main_file else '')
            print(f'    {s.id:8} {s.kind:7} {(s.length or 0):5.1f}s  {lay:10} {var:12} {s.status:8} {s.name}{extra}')
    shows = {}
    for s in film.shots:
        for x in s.shows:
            shows.setdefault(x.split(':')[0].strip(), []).append(s.id)
    if shows:
        print('\n  shows: ' + '; '.join(f'{k} ({", ".join(v)})' for k, v in sorted(shows.items())))
    probs = sb.check(film)
    if probs:
        print()
    for s, msg in probs:
        print(f'  ! {s.id if s else "film"}: {msg}')
    return 1 if probs else 0


@command(options=[IGNORE_CODE])
def status(a):
    """each shot's status, and which takes exist, are current, and why not"""
    film = load(a.board)
    count = {}
    for s in select(film, a.shots):
        cols, whys = [], []
        for tier in ('probe', 'final'):
            for v in (s.variants if s.filmed else []):
                st, why = take_check(film, s, v, tier, a.ignore_code)
                count[st] = count.get(st, 0) + 1
                cols.append(f'{tier}/{v}:{st}')
                if why and why != 'spec changed':
                    whys.append(f'{tier}/{v}: {why}')
        print(f'{s.id:8} {s.status:8} {" ".join(cols)}')
        for w in whys:
            print(f'{"":17}{w}')
    names = {'ok': 'current', 'spec': 'spec changed', 'code': 'code changed', 'unknown': 'code unknown',
             '-': 'not filmed'}
    print('\ntakes: ' + ', '.join(f'{count[k]} {n}' for k, n in names.items() if count.get(k))
          + (' (--ignore-code: code changes not counted)' if a.ignore_code else ''))
    return 0


TAKE_FILES = ('.mkv', '.wav', '.events.json', '.sheet.png', '.log')


def look(spec, other):
    """A spec less what names the take rather than decides how it looks: its film and
    shot, and the scene's path when both name a scene file whose text is compared."""
    drop = {'film', 'shot'}
    if spec.get('scene_ini') is not None and other.get('scene_ini') is not None:
        drop.add('scene')
    return {k: v for k, v in spec.items() if k not in drop}


def source_name(arg):
    """The footage folder a board's takes are in: its `Film:` (or folder) when the board
    can be read here, else the argument as a film name (a board on another branch)."""
    if os.path.isdir(arg) or arg.endswith('.md'):
        try:
            return sb.parse(arg).name
        except sb.BoardError:
            pass
    return os.path.basename(os.path.normpath(arg))


@command(options=[(('--as',), dict(dest='as_id', metavar='ID', help='the shot id in the target board (one shot only)')),
                  (('--tier',), dict(choices=('probe', 'final'), action='append',
                                     help='only this tier (default: both)')),
                  (('--force',), dict(action='store_true', help='replace a target take that is current')),
                  (('--dry-run', '-n'), dict(action='store_true', help='say what would be adopted, copy nothing'))])
def adopt(a):
    """take over another board's takes of a shot whose spec is the same: adopt SRC SHOT... TARGET"""
    if len(a.shots) < 2:
        sys.exit('a2film: adopt <source board> <shot>... <target board>')
    *ids, target = a.shots
    if a.as_id and len(ids) > 1:
        sys.exit('a2film: --as takes one shot')
    dst = load(target)
    src_name = source_name(a.board)
    src_root = os.path.join(sb.a2env.DATA, 'films', src_name)
    adopted = refused = 0
    for sid in ids:
        tid = a.as_id or sid
        try:
            tshot = dst.shot(tid)
        except sb.BoardError as e:
            sys.exit(f'a2film: {dst.name}: {e}')
        if not tshot.filmed:
            sys.exit(f'a2film: {dst.name} {tid} is a {tshot.kind} shot: nothing is filmed for it')
        if src_name == dst.name and sid == tid:
            print(f'{tid}: the same shot of the same film, nothing to adopt')
            continue
        for tier in a.tier or ('probe', 'final'):
            for v in tshot.variants:
                tag = f'{src_name}/{tier}/{sid}/{v} -> {dst.name}/{tier}/{tid}/{v}'
                src = os.path.join(src_root, tier, sid, v)
                if not os.path.exists(src + '.mkv'):
                    continue
                try:
                    rec = json.load(open(src + '.spec.json'))
                except (OSError, ValueError):
                    print(f'{tag}: no readable .spec.json, not adopted')
                    refused += 1
                    continue
                want = dst.spec(tshot, v, tier)
                have = rec.get('spec') or {}
                if look(have, want) != look(want, have):
                    keys = sorted(k for k in set(look(have, want)) | set(look(want, have))
                                  if have.get(k) != want.get(k))
                    print(f'{tag}: the spec differs ({", ".join(keys)}), not adopted')
                    refused += 1
                    continue
                stem = dst.take(tshot, v, tier)
                st, _ = take_check(dst, tshot, v, tier)
                if st == 'ok' and not a.force:
                    print(f'{tag}: the target take is current, kept (--force to replace it)')
                    continue
                print(f'{tag}: adopted' + (' (dry run)' if a.dry_run else ''))
                adopted += 1
                if a.dry_run:
                    continue
                os.makedirs(os.path.dirname(stem), exist_ok=True)
                for ext in TAKE_FILES + ('.spec.json',):
                    if os.path.exists(stem + ext):
                        os.remove(stem + ext)
                for ext in TAKE_FILES:
                    if os.path.exists(src + ext):
                        # a reflink where the file system has them, else a copy: never a hard
                        # link, which a re-filmed take would overwrite in both films
                        subprocess.run(['cp', '--reflink=auto', src + ext, stem + ext], check=True)
                out = dict(rec, hash=dst.spec_hash(tshot, v, tier), spec=want,
                           adopted=dict(film=src_name, shot=sid, hash=rec.get('hash'),
                                        on=time.strftime('%Y-%m-%d %H:%M:%S')))
                with open(stem + '.spec.json', 'w') as f:
                    json.dump(out, f, indent=1)
    print(f'{adopted} takes adopted' + (f', {refused} not adopted' if refused else ''))
    return 1 if refused and not adopted else 0


def main(argv):
    # the other commands register themselves on import
    from . import shoot  # noqa: F401
    try:
        from . import edit  # noqa: F401
    except ImportError:
        pass
    ap = argparse.ArgumentParser(prog='a2film', description='storyboards filmed on the test bench')
    sub = ap.add_subparsers(dest='cmd', required=True)
    for name, fn in COMMANDS.items():
        p = sub.add_parser(name, help=fn.__doc__)
        p.add_argument('board', help='a film folder (films/<name>) or its storyboard.md')
        if fn.shots:
            p.add_argument('shots', nargs='*', help='shot ids (default: all)')
        for args, kw in fn.options:
            p.add_argument(*args, **kw)
    a = ap.parse_args(argv)
    return COMMANDS[a.cmd](a)
