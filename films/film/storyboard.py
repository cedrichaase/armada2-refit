"""The storyboard: a Markdown file parsed into a Film of Sections of Shots.

The format is films/README.md, "The storyboard". The parser keeps everything it does
not read as prose, so the board stays the one document a person edits."""
import hashlib
import json
import os
import re
import sys
from dataclasses import dataclass, field

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, REPO)
import a2env  # noqa: E402

FILM_KEYS = {'Film', 'Final', 'Probe', 'Fps', 'Args', 'Music', 'Pointer', 'Mute', 'Stems'}
SECTION_KEYS = {'Music', 'Mute', 'Stems'}
SHOT_KEYS = {'Kind', 'Shows', 'Scene', 'Args', 'Variants', 'Layout', 'Crop', 'Transition', 'Title', 'Sub',
             'Caption', 'In', 'Length', 'Speed', 'Audio', 'Keys', 'File', 'From', 'Status', 'Background',
             'Run', 'Wallpaper', 'Players', 'Prepare', 'Tier', 'Cut', 'Expect', 'Mute', 'Stems', 'Labels'}
KINDS = {'scene', 'card', 'footage', 'mission', 'desktop'}
LAYOUTS = {'full', 'wipe', 'sweep', 'dissolve', 'split', 'pip'}
STATUSES = {'draft', 'probed', 'approved', 'final'}
# the game's sound categories (testbench/README.md, "Audio stems"): what `Mute:` silences,
# and the stems a take can record; the effects stem is whatever is not a stem of its own
CATEGORIES = ('music', 'voice', 'effects', 'interface')
STEM_CATEGORIES = ('music', 'voice', 'interface')
AUDIO_MODES = {'game', 'none', 'vo'}
OPS = {'scene', 'wait', 'key', 'click', 'step', 'shot', 'ptr', 'drag', 'lclick', 'press', 'run', 'typelog'}

DEFAULT_VARIANTS = {
    'refit': '--install . --install testbench/scene',
    'stock': '--install testbench/scene',
}

FIELD = re.compile(r'^([A-Z][A-Za-z]*)(?: ([A-Za-z0-9_.-]+))?:\s*(.*)$')
FENCE = re.compile(r'^```\s*([A-Za-z]*)\s*$')
SHOT_HEAD = re.compile(r'^([A-Za-z0-9_-]+)\s*(?:[·.:—-]\s*(.*))?$')


class BoardError(Exception):
    pass


@dataclass
class Shot:
    id: str
    name: str
    section: str
    line: int
    fields: dict = field(default_factory=dict)
    setup: list = field(default_factory=list)      # [(line, text)]
    action: list = field(default_factory=list)
    card: list = field(default_factory=list)       # [(style, text)]
    prose: list = field(default_factory=list)
    film: object = field(default=None, repr=False, compare=False)
    file: str = 'storyboard.md'                    # the board file it is written in, relative to the film


    def get(self, k, default=None):
        return self.fields.get(k, default)

    @property
    def kind(self):
        k = self.get('Kind')
        if k:
            return k
        return 'card' if self.card else 'scene'

    @property
    def filmed(self):
        return self.kind in ('scene', 'mission', 'desktop')

    @property
    def variants(self):
        v = self.get('Variants')
        if self.kind == 'desktop':
            return ['desktop']      # no game: one take, a bare desktop
        if self.players:
            return self.players     # one take per player, filmed together
        return [x.strip() for x in v.split(',') if x.strip()] if v else ['refit']

    @property
    def players(self):
        """`Players: host, joiner`: several games in one shot, each a take named after its
        player, all installed as the one variant `Variants:` names (refit by default)."""
        v = self.get('Players')
        return [x.strip() for x in v.split(',') if x.strip()] if v else []

    def install_variant(self, variant):
        """The film variant a take is installed as: its own, or for a player the shot's."""
        if self.players:
            return (self.get('Variants') or 'refit').split(',')[0].strip()
        return variant

    def split_line(self, text):
        """`host: step ...` -> ('host', 'step ...'); a line with no player -> (None, text)."""
        m = re.match(r'^([A-Za-z0-9_-]+):\s+(.*)$', text)
        if m and m.group(1) in self.players:
            return m.group(1), m.group(2)
        return None, text

    @property
    def status(self):
        return self.get('Status', 'draft').split()[0].lower()

    @property
    def tier(self):
        """`Tier:`, the resolution this shot is probed at: `probe` (the film's, the
        default), a height (`1080`, `1440`, `2160`, `4k`) or `WxH`; or `final`, which
        films its final take in a probe run. A size equal to the film's `Final` is `final`:
        the final take is the one worth filming, not a probe that `take` would film again."""
        t = (self.get('Tier') or 'probe').strip().lower()
        if t in ('probe', 'final') or self.film is None:
            return t
        try:
            same = tier_res(t, self.film.res('probe')) == self.film.res('final').strip().lower()
        except BoardError:
            same = False        # `check` reports it
        return 'final' if same else t

    @property
    def labels(self):
        """`Labels: host Player 1; joiner Player 2`: the text a layout shows for a variant
        (or player) in place of its name."""
        out = {}
        for part in (self.get('Labels') or '').split(';'):
            w = part.split(None, 1)
            if len(w) == 2:
                out[w[0]] = w[1].strip()
        return out

    @property
    def ref(self):
        """`From: ref:<id>`: the reference catalogue entry a footage shot uses, or None."""
        f = (self.get('From') or '').split()
        return f[0][4:] if f and f[0].startswith('ref:') else None

    @property
    def length(self):
        v = self.get('Length')
        return float(v) if v else None

    def in_point(self, variant):
        """`In: 1.0` or `In: stock 3.3, refit 4.0`."""
        v = self.get('In', '0').strip()
        if re.fullmatch(r'[0-9.]+', v):
            return float(v)
        for part in v.split(','):
            a = part.split()
            if len(a) == 2 and a[0] == variant:
                return float(a[1])
        return 0.0

    @property
    def layout(self):
        a = self.get('Layout', 'full').split()
        return a[0], a[1:]

    @property
    def transition(self):
        a = self.get('Transition', 'cut').split()
        return a[0], (float(a[1]) if len(a) > 1 else 0.0)

    @property
    def crop(self):
        v = self.get('Crop')
        return [float(x) for x in v.split()] if v else None

    @property
    def shows(self):
        v = self.get('Shows')
        return [x.strip() for x in v.split(',') if x.strip()] if v else []

    def inherited(self, key):
        """A field the shot, its section or the film may set, the nearest winning."""
        if key in self.fields:
            return self.fields[key]
        film = self.film
        if film is None:
            return None
        for sec in film.sections:
            if any(x is self for x in sec.shots):
                if key in sec.fields:
                    return sec.fields[key]
                break
        return film.fields.get(key)

    @property
    def mute(self):
        """`Mute: music, voice`: categories at volume 0 in the take (`none`: nothing)."""
        return categories(self.inherited('Mute'))

    @property
    def stems(self):
        """`Stems: music, voice`: categories recorded as stems of their own; with any, the
        rest of the game's sound is the `effects` stem."""
        return categories(self.inherited('Stems'))

    @property
    def audio(self):
        """`Audio:` as (mode, terms): ('game'|'none'|'vo', []), or ('mix', [(stem, dB)])
        for `Audio: effects + voice -6`."""
        return parse_audio(self.get('Audio'))


def categories(text):
    """'music, voice' -> ['music', 'voice'] (sorted, lower case); 'none' or nothing -> []."""
    words = [w for w in re.split(r'[\s,+]+', (text or '').strip().lower()) if w]
    return [] if words == ['none'] else sorted(set(words))


def parse_audio(text):
    v = (text or 'game').strip().lower()
    if v in AUDIO_MODES:
        return v, []
    terms = []
    for part in v.split('+'):
        a = part.split()
        if not a:
            raise BoardError(f'Audio: {text}: an empty term')
        gain = 0.0
        if len(a) > 1:
            g = a[1][:-2] if a[1].endswith('db') else a[1]
            try:
                gain = float(g)
            except ValueError:
                raise BoardError(f'Audio: {text}: {a[1]} is not a gain in dB')
        if len(a) > 2:
            raise BoardError(f'Audio: {text}: a term is <stem> [gain dB]')
        terms.append((a[0], gain))
    return 'mix', terms


@dataclass
class Section:
    title: str
    line: int
    fields: dict = field(default_factory=dict)
    prose: list = field(default_factory=list)
    shots: list = field(default_factory=list)
    file: str = 'storyboard.md'


@dataclass
class Film:
    path: str           # the storyboard file
    title: str
    fields: dict = field(default_factory=dict)
    variants: dict = field(default_factory=lambda: dict(DEFAULT_VARIANTS))
    prose: list = field(default_factory=list)
    sections: list = field(default_factory=list)
    files: list = field(default_factory=list)      # the files it includes, relative to its folder
    _refs: object = field(default=None, repr=False, compare=False)

    @property
    def dir(self):
        return os.path.dirname(os.path.abspath(self.path))

    @property
    def name(self):
        return self.fields.get('Film') or os.path.basename(self.dir)

    @property
    def shots(self):
        return [s for sec in self.sections for s in sec.shots]

    def shot(self, sid):
        for s in self.shots:
            if s.id == sid:
                return s
        raise BoardError(f'no shot {sid}')

    def footage(self, shot, tier, best=None):
        """A footage shot's file: `From: <shot> [variant]` is that shot's take at this tier
        (the last of its variants by default), or the best one `best(film, shot, variant,
        tier)` names; `From: ref:<id>` the reference's file; `File:` a path under $A2_DATA."""
        if shot.ref:
            r = self.references().get(shot.ref)
            return r.path if r else None
        f = shot.get('From')
        if f:
            a = f.split()
            src = self.shot(a[0])
            v = a[1] if len(a) > 1 else src.variants[-1]
            if best:
                return best(self, src, v, tier)
            return self.take(src, v, tier) + '.mkv'
        f = shot.get('File')
        return os.path.join(a2env.DATA, f) if f else None

    def footage_span(self, shot):
        """(start, end) in the footage file that a shot's `In:` counts from and may run to:
        a reference's `In`/`Out`, else (0, None)."""
        if shot.ref:
            r = self.references().get(shot.ref)
            if r:
                return r.in_point, r.out_point
        return 0.0, None

    def references(self):
        """The reference catalogue beside the films (films/reference/), by id."""
        if self._refs is None:
            self._refs = load_references(os.path.join(os.path.dirname(self.dir), 'reference'))
        return self._refs

    def res(self, tier, shot=None):
        """A take's resolution at this tier: final takes at `Final`; probes at `Probe`, or at
        the shot's own `Tier:`."""
        if tier == 'final':
            return self.fields.get('Final', '3840x2160')
        probe = self.fields.get('Probe', '1280x720')
        if shot is not None and shot.tier not in ('probe', 'final'):
            return tier_res(shot.tier, probe)
        return probe

    def shot_tier(self, shot, tier):
        """The tier a run at `tier` films this shot at: a probe run films a `Tier: final`
        shot's final take, every other shot at its own tier."""
        return 'final' if tier == 'probe' and shot.tier == 'final' else tier

    @property
    def pointer(self):
        """The space pointer coordinates are written in (`Pointer: 1920x1080`), scaled to each
        take's resolution, so a probe and a final take share one action block."""
        w, h = self.fields.get('Pointer', '1920x1080').lower().split('x')
        return int(w), int(h)

    @property
    def fps(self):
        return float(self.fields.get('Fps', 30))

    @property
    def music(self):
        return self.fields.get('Music', '0').strip() == '1'

    def args(self, shot):
        return shot.get('Args') or self.fields.get('Args') or '-nointro a2_borg01'

    def scene_ini(self, shot):
        """The scene file a shot is built from, or None (a mission with Scene: none)."""
        sc = shot.get('Scene')
        if not sc or sc == 'none':
            return None
        if sc.endswith('.ini'):
            return os.path.normpath(os.path.join(self.dir, sc))
        return os.path.join(REPO, 'testbench', 'scene', 'scenes', sc + '.ini')

    def spec(self, shot, variant, tier):
        """Everything that decides what a take of this shot looks like: a take is re-filmed
        only when this changes. Edit fields (Layout, Title, In, ...) are not in it."""
        ini = self.scene_ini(shot)
        ini_text = open(ini).read() if ini and os.path.exists(ini) else None
        return dict(film=self.name, shot=shot.id, variant=variant,
                    install=self.variants.get(shot.install_variant(variant)),
                    players=shot.get('Players'), prepare=shot.get('Prepare'),
                    tier=tier, res=self.res(tier, shot), args=self.args(shot), scene=shot.get('Scene'),
                    scene_ini=ini_text, music=self.music, pointer=self.fields.get('Pointer'),
                    run=shot.get('Run'), wallpaper=shot.get('Wallpaper'),
                    setup=[t for _, t in shot.setup], action=[t for _, t in shot.action],
                    # only when set, so a board that uses neither keeps its takes' hashes
                    **({'mute': shot.mute} if shot.mute else {}),
                    **({'stems': shot.stems} if shot.stems else {}))

    def spec_hash(self, shot, variant, tier):
        s = json.dumps(self.spec(shot, variant, tier), sort_keys=True)
        return hashlib.sha256(s.encode()).hexdigest()[:16]

    def data_dir(self):
        return os.path.join(a2env.DATA, 'films', self.name)

    def take_dir(self, shot, tier):
        return os.path.join(self.data_dir(), tier, shot.id)

    def take(self, shot, variant, tier):
        """The take's files' common stem (add .mkv, .wav, .events.json, .marks.json, .spec.json,
        .sheet.png)."""
        return os.path.join(self.take_dir(shot, tier), variant)


TIER_HEIGHTS = {'4k': 2160, 'uhd': 2160, 'qhd': 1440, 'fhd': 1080, 'hd': 720}


def tier_res(t, probe='1280x720'):
    """`Tier:` as WxH: `1920x1080` as it is; a height (`1080`, `1080p`, `4k`) at the aspect of
    the film's `Probe`, rounded to an even width. Raises BoardError otherwise."""
    t = t.strip().lower()
    if re.fullmatch(r'\d+x\d+', t):
        return t
    h = TIER_HEIGHTS.get(t)
    if h is None and re.fullmatch(r'\d+p?', t):
        h = int(t.rstrip('p'))
    if not h or h < 360:
        raise BoardError(f'Tier {t}: not probe, final, a height (720, 1080, 1440, 2160, 4k) or WxH')
    pw, ph = (int(x) for x in probe.lower().split('x'))
    return f'{round(h * pw / ph / 2) * 2}x{h}'


# ------------------------------------------------------------------ the reference catalogue
REF_KEYS = {'Take', 'File', 'Mission', 'In', 'Out', 'Audio', 'Quality', 'Shows', 'Res'}
QUALITIES = {'good', 'usable', 'poor', 'example'}


@dataclass
class Reference:
    """A scouted reference moment (films/reference/README.md)."""
    id: str
    name: str
    file: str
    line: int
    fields: dict = field(default_factory=dict)
    prose: list = field(default_factory=list)

    def get(self, k, default=None):
        return self.fields.get(k, default)

    @property
    def path(self):
        """The file the moment is in: `Take: <film> <shot> [variant] [tier]` (refit, probe by
        default), a take in $A2_DATA/films/; or `File:`, a path relative to $A2_DATA."""
        t = (self.get('Take') or '').split()
        if t:
            film, shot = t[0], t[1] if len(t) > 1 else ''
            variant = t[2] if len(t) > 2 else 'refit'
            tier = t[3] if len(t) > 3 else 'probe'
            return os.path.join(a2env.DATA, 'films', film, tier, shot, variant + '.mkv')
        f = self.get('File')
        return os.path.join(a2env.DATA, f) if f else None

    @property
    def in_point(self):
        return float(self.get('In') or 0)

    @property
    def out_point(self):
        v = self.get('Out')
        return float(v) if v else None

    @property
    def quality(self):
        return (self.get('Quality') or '').split()[0].rstrip(':,').lower() if self.get('Quality') else ''


def load_references(folder):
    """Every `### <id> · <name>` entry of every .md in the folder but its README, by id."""
    refs = {}
    if not os.path.isdir(folder):
        return refs
    for name in sorted(os.listdir(folder)):
        if not name.endswith('.md') or name.lower() == 'readme.md':
            continue
        path = os.path.join(folder, name)
        cur, fields_open, fence = None, False, False
        for n, raw in enumerate(open(path, encoding='utf-8').read().splitlines(), 1):
            line = raw.rstrip()
            if FENCE.match(line.strip()):
                fence = not fence
                continue
            if fence:
                continue
            if line.startswith('### '):
                h = SHOT_HEAD.match(line[4:].strip())
                if not h:
                    raise BoardError(f'{path}:{n}: a reference heading is "<id> · <name>"')
                if h.group(1) in refs:
                    o = refs[h.group(1)]
                    raise BoardError(f'{path}:{n}: reference {h.group(1)} is used twice (first in {o.file}:{o.line})')
                cur = Reference(id=h.group(1), name=(h.group(2) or '').strip(), file=name, line=n)
                refs[cur.id] = cur
                fields_open = True
                continue
            if line.startswith('#'):
                cur, fields_open = None, False
                continue
            if not cur:
                continue
            f = FIELD.match(line)
            if f and fields_open and f.group(1) in REF_KEYS and not f.group(2):
                cur.fields[f.group(1)] = f.group(3).strip()
                continue
            if line.strip():
                fields_open = False
            cur.prose.append(raw)
    return refs


def check_reference(r):
    """Problems an entry of the catalogue has."""
    out = []
    t = (r.get('Take') or '').split()
    if bool(t) == bool(r.get('File')):
        out.append('a reference names a Take or a File, one of them')
    if t and (len(t) < 2 or (len(t) > 3 and t[3] not in ('probe', 'final'))):
        out.append('Take is <film> <shot> [variant] [probe|final]')
    try:
        i, o = r.in_point, r.out_point
        if o is not None and o <= i:
            out.append(f'Out {o:g} is not after In {i:g}')
    except ValueError:
        out.append('In and Out are seconds')
    if r.quality not in QUALITIES:
        out.append(f'Quality {r.quality or "(none)"} is not one of {", ".join(sorted(QUALITIES))}')
    return out


def board_path(arg):
    """A film folder (its storyboard.md), or the .md itself."""
    if os.path.isdir(arg):
        return os.path.join(arg, 'storyboard.md')
    return arg


INCLUDE = re.compile(r'^Include:\s*(\S+\.md)\s*$')


def expand(path, film_dir, stack=()):
    """The board's lines with every `Include: <file>.md` spliced in where it stands, as
    [(file, line number, text)]. An included file is read from its first `##` or `###`
    heading or `Include:`: what stands before it is that file's own preamble, for its
    reader. Files are relative to the file that includes them and must lie in the film's
    folder."""
    real = os.path.realpath(path)
    if real in stack:
        raise BoardError(f'{path}: included in itself ({" -> ".join(os.path.relpath(x, film_dir) for x in stack + (real,))})')
    try:
        lines = open(path, encoding='utf-8').read().splitlines()
    except OSError as e:
        raise BoardError(f'{path}: cannot read it ({e.strerror})')
    out = []
    included = bool(stack)
    started = not included
    fence = None
    for n, raw in enumerate(lines, 1):
        line = raw.rstrip()
        m = FENCE.match(line.strip())
        if not started and not fence and (line.startswith('## ') or line.startswith('### ')):
            started = True
        if fence:
            if m and not m.group(1):
                fence = None
            if started:
                out.append((path, n, raw))
            continue
        if m:
            fence = n
            if started:
                out.append((path, n, raw))
            continue
        inc = INCLUDE.match(line)
        if inc:
            sub = os.path.normpath(os.path.join(os.path.dirname(path), inc.group(1)))
            rel = os.path.relpath(os.path.realpath(sub), os.path.realpath(film_dir))
            if rel.startswith('..'):
                raise BoardError(f'{path}:{n}: Include {inc.group(1)}: not in the film\'s folder')
            if not os.path.exists(sub):
                raise BoardError(f'{path}:{n}: Include {inc.group(1)}: no such file')
            got = expand(sub, film_dir, stack + (real,))
            if not got:
                raise BoardError(f'{path}:{n}: Include {inc.group(1)}: it has no ## section or ### shot')
            out += got
            started = True      # what follows an Include: is read, as after a heading
            continue
        if not started:
            continue
        if included and line.startswith('# '):
            raise BoardError(f'{path}:{n}: an included file has no film heading (# ...) after its first section')
        out.append((path, n, raw))
    if fence:
        raise BoardError(f'{path}:{fence}: a fenced block is never closed')
    return out


def parse(path):
    path = board_path(path)
    if not os.path.exists(path):
        raise BoardError(f'no storyboard at {path}')
    film_dir = os.path.dirname(os.path.abspath(path))
    film = None
    sec = None
    shot = None
    fence = None          # (kind, start line) while inside a fenced block
    owner_fields = True   # Key: lines still count as fields of the current heading
    seen = {}             # shot id -> where it is
    for fpath, n, raw in expand(path, film_dir):
        where = f'{fpath}:{n}'
        line = raw.rstrip()
        m = FENCE.match(line.strip())
        if fence:
            if m and not m.group(1):
                fence = None
                continue
            kind = fence[0]
            if shot and kind in ('setup', 'action'):
                t = line.split('#', 1)[0].strip()
                if t:
                    getattr(shot, kind).append((n, t))
            elif shot and kind == 'card':
                t = line.strip()
                if t:
                    st = re.match(r'^\[(\w+)\]\s*(.*)$', t)
                    shot.card.append((st.group(1), st.group(2)) if st else ('heading', t))
            else:
                (shot or sec or film).prose.append(raw)
            continue
        if m and m.group(1) in ('setup', 'action', 'card'):
            if not shot:
                raise BoardError(f'{where}: a ```{m.group(1)} block outside a shot')
            fence = (m.group(1), n)
            continue
        if m:
            fence = ('', n)
            if film:
                (shot or sec or film).prose.append(raw)
            continue
        if line.startswith('# '):
            if film:
                raise BoardError(f'{where}: a second film heading')
            film = Film(path=path, title=line[2:].strip())
            sec, shot, owner_fields = None, None, True
            continue
        if not film:
            continue
        rel = os.path.relpath(fpath, film_dir)
        if fpath != path and rel not in film.files:
            film.files.append(rel)
        if line.startswith('## '):
            sec = Section(title=line[3:].strip(), line=n, file=rel)
            film.sections.append(sec)
            shot, owner_fields = None, True
            continue
        if line.startswith('### '):
            if not sec:
                sec = Section(title='', line=n, file=rel)
                film.sections.append(sec)
            h = SHOT_HEAD.match(line[4:].strip())
            if not h:
                raise BoardError(f'{where}: a shot heading is "<id> · <name>"')
            shot = Shot(id=h.group(1), name=(h.group(2) or '').strip(), section=sec.title, line=n, film=film,
                        file=rel)
            if shot.id in seen:
                raise BoardError(f'{where}: shot id {shot.id} is used twice (first at {seen[shot.id]})')
            seen[shot.id] = os.path.relpath(fpath) + f':{n}'
            sec.shots.append(shot)
            owner_fields = True
            continue
        f = FIELD.match(line)
        if f and owner_fields:
            key, arg, val = f.group(1), f.group(2), f.group(3).strip()
            if shot:
                if key in SHOT_KEYS and not arg:
                    shot.fields[key] = val
                    continue
            elif sec:
                if key in SECTION_KEYS and not arg:
                    sec.fields[key] = val
                    continue
            else:
                if key == 'Variant' and arg:
                    film.variants[arg] = val
                    continue
                if key in FILM_KEYS and not arg:
                    film.fields[key] = val
                    continue
        # a field block ends at the first line of prose after it (blank lines don't end it)
        if line.strip():
            owner_fields = False
        (shot or sec or film).prose.append(raw)
    if not film:
        raise BoardError(f'{path}: no "# Title" heading')
    return film


def stem_names(shot):
    """The stems a take of this shot has: its Stems and `effects`, the rest; none without."""
    return [c for c in shot.stems if c != 'effects'] + ['effects'] if shot.stems else []


def audio_problems(film, s):
    out = []
    mute, stems = s.mute, s.stems
    for key, cats, allowed in (('Mute', mute, CATEGORIES), ('Stems', stems, STEM_CATEGORIES)):
        bad = [c for c in cats if c not in allowed]
        if bad and key in s.fields:
            out.append(f'{key} {", ".join(bad)}: not one of {", ".join(allowed)}')
    both = set(mute) & set(stems)
    if both:
        out.append(f'{", ".join(sorted(both))}: both in Mute and in Stems')
    if (mute or stems) and s.kind == 'desktop' and ('Mute' in s.fields or 'Stems' in s.fields):
        out.append('a desktop shot has no game sound to mute or split')
    try:
        mode, terms = s.audio
    except BoardError as e:
        return out + [str(e)]
    if mode == 'mix':
        src = s
        if s.kind == 'footage' and s.ref:
            # a reference's stems are the files beside its take (`<variant>.music.wav`)
            r = film.references().get(s.ref)
            if r is None:
                return out          # the unknown id is reported on its own
            p = r.path or ''
            have = [c for c in CATEGORIES if p.endswith('.mkv')
                    and os.path.exists(p[:-4] + f'.{c}.wav')]
            label = f'ref:{s.ref}'
        else:
            if s.kind == 'footage':
                a = (s.get('From') or '').split()
                src = next((x for x in film.shots if a and x.id == a[0]), None)
                if src is None:
                    return out + ['an Audio mix of stems needs a From: shot that records them']
            have, label = stem_names(src), src.id
        for name, _ in terms:
            if name == 'game':
                continue
            if not have:
                out.append(f'Audio: {name}: {label} records no stems (Stems: ...)')
                break
            if name not in have:
                out.append(f'Audio: {name} is not a stem of {label} ({", ".join(have)}, or game)')
    return out


def check(film):
    """Problems a board has, as (shot or None, message); empty when it can be filmed."""
    out = []
    try:
        refs = film.references() if any(s.ref for s in film.shots) else {}
    except BoardError as e:
        refs = {}
        out.append((None, str(e)))
    for s in film.shots:
        k = s.kind
        if k not in KINDS:
            out.append((s, f'Kind {k} is not one of {", ".join(sorted(KINDS))}'))
        if s.status not in STATUSES:
            out.append((s, f'Status {s.status} is not one of {", ".join(sorted(STATUSES))}'))
        lay, _ = s.layout
        if lay not in LAYOUTS:
            out.append((s, f'Layout {lay} is not one of {", ".join(sorted(LAYOUTS))}'))
        if s.length is None and k != 'footage':
            out.append((s, 'no Length'))
        if k == 'card' and not s.card:
            out.append((s, 'a card with no ```card block'))
        if k == 'footage' and not (s.get('File') or s.get('From')):
            out.append((s, 'footage with no File or From'))
        if s.ref is not None:
            r = refs.get(s.ref)
            if not r:
                out.append((s, f'From ref:{s.ref}: no such entry in films/reference/'))
            elif r.quality == 'example':
                out.append((s, f'From ref:{s.ref}: an example entry, not a real reference'))
        elif s.get('From'):
            a = s.get('From').split()
            src = next((x for x in film.shots if x.id == a[0]), None)
            if not src or not src.filmed:
                out.append((s, f'From {a[0]}: no filmed shot of that id'))
            elif len(a) > 1 and a[1] not in src.variants:
                out.append((s, f'From {a[0]} {a[1]}: {a[0]} films no {a[1]}'))
        if k == 'desktop':
            if any(t.split()[0] == 'run' for _, t in s.setup + s.action) and not s.get('Run'):
                out.append((s, 'a run command with no Run: field'))
            if s.get('Scene'):
                out.append((s, 'a desktop shot has no Scene: there is no game'))
            w = (s.get('Wallpaper') or '').split()
            if w and not w[0].startswith('#'):
                src = next((x for x in film.shots if x.id == w[0]), None)
                if not src or not src.filmed or src.kind == 'desktop':
                    out.append((s, f'Wallpaper {w[0]}: no filmed game shot of that id'))
                elif len(w) > 1 and w[1] not in src.variants:
                    out.append((s, f'Wallpaper {w[0]} {w[1]}: {w[0]} films no {w[1]}'))
        if s.filmed:
            for v in {s.install_variant(v) for v in s.variants}:
                if v not in film.variants and k != 'desktop':
                    out.append((s, f'variant {v} is not defined (Variant {v}: ...)'))
            if s.players and len(s.players) < 2:
                out.append((s, 'Players names fewer than two players'))
            if s.get('Prepare') and not os.path.exists(os.path.join(REPO, s.get('Prepare').split()[0])):
                out.append((s, f'Prepare: no script {s.get("Prepare").split()[0]} in the repository'))
            ini = film.scene_ini(s)
            if k == 'scene' and not ini:
                out.append((s, 'a scene shot with no Scene'))
            if ini and not os.path.exists(ini):
                out.append((s, f'no scene file {ini}'))
            if not s.action:
                out.append((s, 'no ```action block'))
            for ln, t in s.setup + s.action:
                op = s.split_line(t)[1].split()[0]
                if op not in OPS:
                    out.append((s, f'line {ln}: unknown command {op}'))
            if lay in ('wipe', 'sweep', 'dissolve', 'split', 'pip') and len(s.variants) < 2:
                out.append((s, f'Layout {lay} needs two variants'))
            # a step that waits for text on screen takes as long as the game does: no telling
            ocr_wait = any(re.match(r'step\s+wait\b.*\bfor\s+"', s.split_line(t)[1], re.I) for _, t in s.action)
            if s.length is not None and not ocr_wait:
                act = sum(float(t.split()[1]) for t in (s.split_line(x)[1] for _, x in s.action) if t.startswith('wait '))
                need = s.length * float(s.get('Speed', 1) or 1)    # seconds of take on screen
                for v in s.variants:
                    if s.in_point(v) + need > act + 0.01:
                        out.append((s, f'{v}: In {s.in_point(v)} + {need:g} s of take is past the action\'s '
                                       f'{act:g} s of waits'))
        if s.get('Tier'):
            if not s.filmed:
                out.append((s, f'Tier on a {k} shot: only filmed shots have a tier'))
            elif s.tier not in ('probe', 'final'):
                try:
                    tier_res(s.tier, film.res('probe'))
                except BoardError as e:
                    out.append((s, str(e)))
        # Cut: and Expect: are read against a take's marks; here only their spelling
        from . import marks
        for key, parse in (('Cut', marks.parse_cut), ('Expect', marks.parse_expect)):
            if s.get(key):
                if not s.filmed:
                    out.append((s, f'{key}: only a filmed shot has marks'))
                    continue
                try:
                    parse(s.get(key))
                except marks.MarkError as e:
                    out.append((s, f'{key}: {e}'))
        out += [(s, m) for m in audio_problems(film, s)]
        c = s.crop
        if c is not None and (len(c) != 4 or any(x < 0 or x > 1 for x in c)):
            out.append((s, 'Crop is x y w h, four fractions of the frame'))
    used = {s.ref for s in film.shots if s.ref}
    for r in refs.values():
        if r.id in used:
            for msg in check_reference(r):
                out.append((None, f'reference {r.id} ({r.file}:{r.line}): {msg}'))
    for owner, f in [(f'section {sec.title}', sec.fields) for sec in film.sections] + [('film', film.fields)]:
        for key, allowed in (('Mute', CATEGORIES), ('Stems', STEM_CATEGORIES)):
            bad = [c for c in categories(f.get(key)) if c not in allowed]
            if bad:
                out.append((None, f'{owner}: {key} {", ".join(bad)}: not one of {", ".join(allowed)}'))
    ids = {s.id for s in film.shots}
    for sec in film.sections:
        m = re.search(r'\buntil\s+(\S+)', sec.fields.get('Music', ''))
        if m and m.group(1) not in ids:
            out.append((None, f'section {sec.title}: Music until {m.group(1)}: no such shot'))
    return out
