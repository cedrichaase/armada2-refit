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

FILM_KEYS = {'Film', 'Final', 'Probe', 'Fps', 'Args', 'Music'}
SECTION_KEYS = {'Music'}
SHOT_KEYS = {'Kind', 'Shows', 'Scene', 'Args', 'Variants', 'Layout', 'Crop', 'Transition', 'Title', 'Sub',
             'Caption', 'In', 'Length', 'Speed', 'Audio', 'Keys', 'File', 'From', 'Status', 'Background'}
KINDS = {'scene', 'card', 'footage', 'mission'}
LAYOUTS = {'full', 'wipe', 'dissolve', 'split', 'pip'}
STATUSES = {'draft', 'probed', 'approved', 'final'}
OPS = {'scene', 'wait', 'key', 'click', 'step', 'shot', 'ptr', 'drag', 'lclick', 'press'}

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
        return self.kind in ('scene', 'mission')

    @property
    def variants(self):
        v = self.get('Variants')
        return [x.strip() for x in v.split(',') if x.strip()] if v else ['refit']

    @property
    def status(self):
        return self.get('Status', 'draft').split()[0].lower()

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


@dataclass
class Section:
    title: str
    line: int
    fields: dict = field(default_factory=dict)
    prose: list = field(default_factory=list)
    shots: list = field(default_factory=list)


@dataclass
class Film:
    path: str           # the storyboard file
    title: str
    fields: dict = field(default_factory=dict)
    variants: dict = field(default_factory=lambda: dict(DEFAULT_VARIANTS))
    prose: list = field(default_factory=list)
    sections: list = field(default_factory=list)

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

    def footage(self, shot, tier):
        """A footage shot's file: `From: <shot> [variant]` is that shot's take at this tier
        (the last of its variants by default); `File:` a path under $A2_DATA."""
        f = shot.get('From')
        if f:
            a = f.split()
            src = self.shot(a[0])
            return self.take(src, a[1] if len(a) > 1 else src.variants[-1], tier) + '.mkv'
        f = shot.get('File')
        return os.path.join(a2env.DATA, f) if f else None

    def res(self, tier):
        return self.fields.get('Final', '3840x2160') if tier == 'final' else self.fields.get('Probe', '1280x720')

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
        return dict(film=self.name, shot=shot.id, variant=variant, install=self.variants.get(variant),
                    tier=tier, res=self.res(tier), args=self.args(shot), scene=shot.get('Scene'),
                    scene_ini=ini_text, music=self.music,
                    setup=[t for _, t in shot.setup], action=[t for _, t in shot.action])

    def spec_hash(self, shot, variant, tier):
        s = json.dumps(self.spec(shot, variant, tier), sort_keys=True)
        return hashlib.sha256(s.encode()).hexdigest()[:16]

    def data_dir(self):
        return os.path.join(a2env.DATA, 'films', self.name)

    def take_dir(self, shot, tier):
        return os.path.join(self.data_dir(), tier, shot.id)

    def take(self, shot, variant, tier):
        """The take's files' common stem (add .mkv, .wav, .events.json, .spec.json, .sheet.png)."""
        return os.path.join(self.take_dir(shot, tier), variant)


def board_path(arg):
    """A film folder (its storyboard.md), or the .md itself."""
    if os.path.isdir(arg):
        return os.path.join(arg, 'storyboard.md')
    return arg


def parse(path):
    path = board_path(path)
    if not os.path.exists(path):
        raise BoardError(f'no storyboard at {path}')
    film = None
    sec = None
    shot = None
    fence = None          # (kind, start line) while inside a fenced block
    owner_fields = True   # Key: lines still count as fields of the current heading
    lines = open(path, encoding='utf-8').read().splitlines()
    for n, raw in enumerate(lines, 1):
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
                raise BoardError(f'{path}:{n}: a ```{m.group(1)} block outside a shot')
            fence = (m.group(1), n)
            continue
        if m:
            fence = ('', n)
            if film:
                (shot or sec or film).prose.append(raw)
            continue
        if line.startswith('# '):
            if film:
                raise BoardError(f'{path}:{n}: a second film heading')
            film = Film(path=path, title=line[2:].strip())
            sec, shot, owner_fields = None, None, True
            continue
        if not film:
            continue
        if line.startswith('## '):
            sec = Section(title=line[3:].strip(), line=n)
            film.sections.append(sec)
            shot, owner_fields = None, True
            continue
        if line.startswith('### '):
            if not sec:
                sec = Section(title='', line=n)
                film.sections.append(sec)
            h = SHOT_HEAD.match(line[4:].strip())
            if not h:
                raise BoardError(f'{path}:{n}: a shot heading is "<id> · <name>"')
            shot = Shot(id=h.group(1), name=(h.group(2) or '').strip(), section=sec.title, line=n, film=film)
            if any(s.id == shot.id for s in film.shots):
                raise BoardError(f'{path}:{n}: shot id {shot.id} is used twice')
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
    if fence:
        raise BoardError(f'{path}:{fence[1]}: a fenced block is never closed')
    if not film:
        raise BoardError(f'{path}: no "# Title" heading')
    return film


def check(film):
    """Problems a board has, as (shot or None, message); empty when it can be filmed."""
    out = []
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
        if s.get('From'):
            a = s.get('From').split()
            src = next((x for x in film.shots if x.id == a[0]), None)
            if not src or not src.filmed:
                out.append((s, f'From {a[0]}: no filmed shot of that id'))
            elif len(a) > 1 and a[1] not in src.variants:
                out.append((s, f'From {a[0]} {a[1]}: {a[0]} films no {a[1]}'))
        if s.filmed:
            for v in s.variants:
                if v not in film.variants:
                    out.append((s, f'variant {v} is not defined (Variant {v}: ...)'))
            ini = film.scene_ini(s)
            if k == 'scene' and not ini:
                out.append((s, 'a scene shot with no Scene'))
            if ini and not os.path.exists(ini):
                out.append((s, f'no scene file {ini}'))
            if not s.action:
                out.append((s, 'no ```action block'))
            for ln, t in s.setup + s.action:
                op = t.split()[0]
                if op not in OPS:
                    out.append((s, f'line {ln}: unknown command {op}'))
            if lay in ('wipe', 'dissolve', 'split', 'pip') and len(s.variants) < 2:
                out.append((s, f'Layout {lay} needs two variants'))
            if s.length is not None:
                act = sum(float(t.split()[1]) for _, t in s.action if t.startswith('wait '))
                need = s.length * float(s.get('Speed', 1) or 1)    # seconds of take on screen
                for v in s.variants:
                    if s.in_point(v) + need > act + 0.01:
                        out.append((s, f'{v}: In {s.in_point(v)} + {need:g} s of take is past the action\'s '
                                       f'{act:g} s of waits'))
        c = s.crop
        if c is not None and (len(c) != 4 or any(x < 0 or x > 1 for x in c)):
            out.append((s, 'Crop is x y w h, four fractions of the frame'))
    ids = {s.id for s in film.shots}
    for sec in film.sections:
        m = re.search(r'\buntil\s+(\S+)', sec.fields.get('Music', ''))
        if m and m.group(1) not in ids:
            out.append((None, f'section {sec.title}: Music until {m.group(1)}: no such shot'))
    return out
