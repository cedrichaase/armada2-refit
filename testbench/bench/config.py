"""Paths, resolutions and knobs.  Everything overridable from the environment."""
import os
import sys
from pathlib import Path

HOME = Path.home()
BENCH = Path(__file__).resolve().parent.parent          # testbench/
REPO = BENCH.parent                                     # the checkout the bench runs from
A2TEST = REPO / 'a2test'

sys.path.insert(0, str(REPO))
import a2env  # noqa: E402  (the repository root: where the game is, a2env.sh has the rules)
GAME = Path(a2env.GAME)
PREFIX = Path(a2env.PREFIX)
PROTON = Path(a2env.PROTON)
# Where the private repository's assets and builds are. Resolved HERE, before a session
# points XDG_DATA_HOME at its own scratch dir -- A2_DATA's default hangs off it, so a
# stacked private ./install would otherwise look in an empty directory.
DATA = Path(a2env.DATA)
UMU = Path(os.environ.get('A2_UMU', HOME / '.config/heroic/tools/runtimes/umu/umu_run.py'))
VKBASALT = Path(os.environ.get('XDG_DATA_HOME', HOME / '.local/share')) / 'a2-vkbasalt'

# Clones are big (a reflink copy of 5 GB -- free on btrfs, but it must be the SAME
# filesystem as the game) and transient.  Results are small and kept.
CACHE = Path(os.environ.get('A2TEST_CACHE', HOME / '.cache/a2test'))
RESULTS = Path(os.environ.get('A2TEST_RESULTS', HOME / '.local/share/a2test/results'))

INPUT_SRC = BENCH / 'input'
INPUT_BIN = INPUT_SRC / 'a2input'

# The overrides Heroic launches with (GamesConfig/1174788223.json).
DLL_OVERRIDES = 'winmm=n,b;d3d8=n,b;d3d9=n,b'

# "Every relevant aspect ratio".  21:9 is the resolution the project is played at.
ASPECTS = {
    '4:3': (1600, 1200),
    '5:4': (1280, 1024),
    '16:10': (1920, 1200),
    '16:9': (1920, 1080),
    '21:9': (3440, 1440),
}
DEFAULT_ASPECTS = ['4:3', '16:10', '16:9', '21:9']
REFERENCE_ASPECT = '4:3'   # the stock game's native shape: nothing is stretched at 4:3

JOBS = int(os.environ.get('A2TEST_JOBS', 5))   # game instances at once (a2test run --jobs)
VNC_BASE_PORT = int(os.environ.get('A2TEST_VNC_PORT', 5910))
JUDGE_MODEL = os.environ.get('A2TEST_MODEL', 'sonnet')   # judged and agent steps; '' = claude's own default


def parse_res(s):
    """'16:9' -> (1920, 1080); '2560x1080' -> (2560, 1080)."""
    s = s.strip().lower()
    if s in ASPECTS:
        return ASPECTS[s]
    if 'x' in s:
        w, h = s.split('x', 1)
        return int(w), int(h)
    raise ValueError(f'not a resolution or known aspect: {s!r} (known: {", ".join(ASPECTS)})')


def res_name(res):
    return f'{res[0]}x{res[1]}'


def aspect_name(res):
    for k, v in ASPECTS.items():
        if v[0] * res[1] == v[1] * res[0]:
            return k
    return f'{res[0] / res[1]:.2f}:1'
