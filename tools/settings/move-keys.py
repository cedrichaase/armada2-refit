#!/usr/bin/env python3
"""Moves the plugin-ini keys that a settings row says it took over (`moved_from =
"<Stem>.<Section>.<Key>"` in a settings.toml; tools/settings/README.md, "Moved keys").

    move-keys.py [--repo DIR] GAME_DIR

For every row with `moved_from`, in the game directory's `<Stem>.ini`: when it holds
`[Section] Key`, a value other than the row's default goes to the row's own ini and key,
and the old key is removed either way. Run by ./install after every layer, so a value a
player set under the old name survives; a second run finds nothing to do. The edit is
a2ini's (host.sh); a file that does not exist is left alone, with a line saying so.
The overlay is the UI hub's to move (`ui/src/settings.rs`), not this script's.
"""
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen  # noqa: E402
import inival  # noqa: E402


def split_moved(moved):
    """`Stem.Section.Key` -> (stem, section, key): the section may hold dots."""
    stem, rest = moved.split('.', 1)
    sec, key = rest.rsplit('.', 1)
    return stem, sec, key


def host():
    """The built a2ini (host.sh builds it on first use)."""
    r = subprocess.run([os.path.join(os.path.dirname(os.path.abspath(__file__)), 'host.sh')],
                       stdout=subprocess.PIPE, text=True)
    if r.returncode != 0:
        raise SystemExit('move-keys: tools/settings/host.sh could not build a2ini')
    return r.stdout.strip()


def main(argv):
    repo = gen.ROOT
    if argv[:1] == ['--repo'] and len(argv) >= 2:
        repo, argv = os.path.abspath(argv[1]), argv[2:]
    if len(argv) != 1:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    game = argv[0]
    a2ini = None
    for path in gen.walk_tables(repo):
        try:
            rows = gen.rows_of(gen.load(path), path)
        except gen.TableError as e:
            print(f'move-keys: {e}', file=sys.stderr)
            return 1
        for _, r in rows:
            if r['moved_from'] is None:
                continue
            stem, osec, okey = split_moved(r['moved_from'])
            old = os.path.join(game, stem + '.ini')
            new = os.path.join(game, r['file'])
            v = inival.value(inival.read(old), osec, okey) if os.path.isfile(old) else None
            if v is None:
                continue
            if a2ini is None:
                a2ini = host()
            if v.lower() != r['def'].lower():
                if not os.path.isfile(new):
                    print(f'move-keys: {r["file"]} is not there; {stem}.ini [{osec}] {okey}={v} stays')
                    continue
                subprocess.run([a2ini, 'edit', new, f'{r["section"]}:{r["key"]}={v}'], check=True, stdout=subprocess.DEVNULL)
                print(f'move-keys: moved {stem}.ini [{osec}] {okey}={v} to {r["file"]} [{r["section"]}] {r["key"]}')
            else:
                print(f'move-keys: dropped {stem}.ini [{osec}] {okey}={v} (the default of {r["file"]} [{r["section"]}] {r["key"]})')
            subprocess.run([a2ini, 'unset', old, f'{osec}:{okey}'], check=True, stdout=subprocess.DEVNULL)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
