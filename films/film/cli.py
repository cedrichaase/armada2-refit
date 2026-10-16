"""./a2film -- the commands (films/README.md)."""
import argparse
import json
import os
import sys

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


def take_state(film, shot, variant, tier):
    """'-' no take, 'ok' current, 'stale' filmed from a spec that has since changed."""
    stem = film.take(shot, variant, tier)
    if not os.path.exists(stem + '.mkv'):
        return '-'
    try:
        old = json.load(open(stem + '.spec.json')).get('hash')
    except (OSError, ValueError):
        old = None
    return 'ok' if old == film.spec_hash(shot, variant, tier) else 'stale'


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


@command
def status(a):
    """each shot's status, and which takes exist and are current"""
    film = load(a.board)
    for s in select(film, a.shots):
        cols = [f'{tier}/{v}:{take_state(film, s, v, tier)}'
                for tier in ('probe', 'final') for v in s.variants] if s.filmed else []
        print(f'{s.id:8} {s.status:8} {" ".join(cols)}')
    return 0


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
