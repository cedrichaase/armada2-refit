"""Which code a take shows (films/README.md, "When a take is current").

A take is filmed from a checkout at some commit, perhaps with uncommitted changes. It
still shows the game as it is now only if no file the take depends on has changed since:
the take records its commit and a hash of its uncommitted changes to those files, and is
current while the same diff, taken from that commit to today's working tree, hashes the
same. Files that cannot change what a take shows (docs, the films themselves, offline
tools) are left out, so a change to them keeps every take."""
import hashlib
import os
import shlex
import subprocess

from . import storyboard as sb

# never part of what a take shows: docs, the boards and this tool, agent and CI config,
# the publish check, the bench's test scenarios, render3d's offline measurement tools
EXCLUDE = ['films', '.claude', '.github', 'publish', 'testbench/scenarios', 'render3d/perf']
EXCLUDE_GLOB = ['**/*.md', '**/.gitignore', 'LICENSE']
# what every take runs through besides its installs: the bench, and a2mod (`a2mod stock`
# before the installs) with the environment they read
BENCH = ['testbench', 'a2test', 'a2mod', 'a2env.sh', 'a2env.py']

_memo = {}


def git(repo, *args, binary=False):
    r = subprocess.run(['git', '-C', repo, *args], capture_output=True, text=not binary)
    return r.returncode, r.stdout


def scope(film, shot, variant, repo=sb.REPO):
    """The repository paths a take of this shot and variant depends on, relative to the
    repository: just its installs and the bench when every install is a folder of this
    repository (the stock variant: `testbench/scene`), else the whole repository ('.')."""
    if shot.kind == 'desktop' or shot.get('Run') or shot.get('Prepare'):
        return ('.',)
    words = shlex.split(film.variants.get(shot.install_variant(variant)) or '')
    paths = []
    for i, w in enumerate(words):
        if w == '--install' and i + 1 < len(words):
            p = os.path.relpath(os.path.realpath(os.path.join(repo, words[i + 1])), os.path.realpath(repo))
            if p == '.' or p.startswith('..'):
                return ('.',)      # the whole checkout, or another one we cannot follow
            paths.append(p)
    return tuple(sorted(set(BENCH + paths)))


def pathspec(paths):
    return (list(paths) + [f':(exclude){p}' for p in EXCLUDE]
            + [f':(exclude,glob){g}' for g in EXCLUDE_GLOB])


def diff_hash(repo, base, paths, fresh=False):
    """A hash of the diff from `base` (a commit) to the working tree, over `paths`; ''
    when there is none. Untracked files are not seen. Kept per commit and scope for the
    run of one command, unless `fresh`."""
    key = (repo, base, tuple(paths))
    if fresh or key not in _memo:
        rc, out = git(repo, 'diff', '--binary', '--full-index', '--no-renames', '--no-color',
                      '--no-ext-diff', '--no-textconv', '--diff-algorithm=myers', base, '--',
                      *pathspec(paths), binary=True)
        if rc != 0:
            raise RuntimeError(f'git diff {base} failed')
        _memo[key] = hashlib.sha256(out).hexdigest()[:16] if out else ''
    return _memo[key]


def head(repo=sb.REPO):
    rc, out = git(repo, 'rev-parse', 'HEAD')
    return out.strip() if rc == 0 else None


def record(film, shot, variant, repo=sb.REPO):
    """What a take stores of the code it was filmed from (in its .spec.json), taken as
    its session starts, when the variant is installed."""
    c = head(repo)
    paths = scope(film, shot, variant, repo)
    rc, _ = git(repo, 'diff', '--quiet', 'HEAD', '--')
    return dict(commit=c, dirty=rc != 0, scope=list(paths),
                code=diff_hash(repo, c, paths, fresh=True) if c else None)


def resolve(repo, rev):
    key = (repo, 'rev', rev)
    if key not in _memo:
        rc, out = git(repo, 'rev-parse', '--verify', '-q', f'{rev}^{{commit}}')
        _memo[key] = out.strip() if rc == 0 else None
    return _memo[key]


def changed_dirs(repo, base, paths):
    """The top-level folders (or files) changed between `base` and HEAD, over `paths`."""
    key = (repo, 'dirs', base, tuple(paths))
    if key not in _memo:
        _, out = git(repo, 'diff', '--name-only', '--no-renames', base, 'HEAD', '--', *pathspec(paths))
        _memo[key] = sorted({f.split('/')[0] for f in out.split()})
    return _memo[key]


def short(rev):
    return (rev or '')[:7]


def check(spec, film, shot, variant, repo=sb.REPO):
    """('ok', None), or ('code', why) when the code it shows changed, or ('unknown', why)
    when the take does not say which code it was filmed from."""
    rev = spec.get('commit')
    if not rev:
        return 'unknown', 'no commit recorded'
    legacy = 'code' not in spec          # before films 1.1.0: `git describe --always --dirty`
    dirty = rev.endswith('-dirty') if legacy else spec.get('dirty')
    rev = rev[:-len('-dirty')] if rev.endswith('-dirty') else rev
    full = resolve(repo, rev)
    if not full:
        return 'unknown', f'commit {short(rev)} is not in this repository'
    paths = scope(film, shot, variant, repo)
    now = diff_hash(repo, full, paths)
    then = '' if legacy else (spec.get('code') or '')
    if now == then and not (legacy and dirty):
        return 'ok', None
    dirs = changed_dirs(repo, full, paths)
    if dirs:
        names = ', '.join(dirs[:5]) + (f' and {len(dirs) - 5} more' if len(dirs) > 5 else '')
        return 'code', f'code changed since {short(full)} ({names})'
    if now:
        if then:
            return 'code', f'uncommitted code changes, other than at the take ({short(full)}, dirty)'
        return 'code', f'uncommitted code changes since {short(full)}'
    if legacy:
        return 'unknown', f'filmed from uncommitted changes to {short(full)}, not recorded which'
    return 'code', f'filmed from uncommitted changes to {short(full)}, since undone'
