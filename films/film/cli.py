"""./a2film -- the commands (films/README.md)."""
import argparse
import json
import os
import sys

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
    for sec in film.sections:
        print(f'\n  {sec.title}' + (f'   [music: {sec.fields["Music"]}]' if 'Music' in sec.fields else ''))
        for s in sec.shots:
            lay = ' '.join([s.layout[0]] + s.layout[1])
            var = ','.join(s.variants) if s.filmed else ''
            print(f'    {s.id:8} {s.kind:7} {(s.length or 0):5.1f}s  {lay:10} {var:12} {s.status:8} {s.name}')
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


def main(argv):
    if argv and argv[0] == 'slots':       # the machine-wide game slots: no board needed
        sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), 'testbench'))
        from bench import slots
        return slots.main()
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
