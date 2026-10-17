"""The run report: index.html (to look at), report.md (to read or paste), results.json
(for tools).  Everything a run produced is linked relative to the run directory, so the
directory can be moved or archived whole."""
import html
import json
import re
from pathlib import Path

STATUS_ORDER = ['fail', 'error', 'review', 'inconclusive', 'pass', 'ok', 'skip']
BADGE = {'pass': 'PASS', 'fail': 'FAIL', 'error': 'ERROR', 'review': 'REVIEW', 'inconclusive': 'INCONCLUSIVE',
         'ok': 'done', 'skip': 'skipped'}


def collect(run_dir):
    run_dir = Path(run_dir)
    cases = []
    for rj in sorted(run_dir.glob('*/*/result.json')):
        r = json.loads(rj.read_text())
        r['dir'] = str(rj.parent.relative_to(run_dir))
        cases.append(r)
    return cases


def summary_counts(cases):
    c = {}
    for x in cases:
        c[x['status']] = c.get(x['status'], 0) + 1
    return c


def write(run_dir, meta):
    run_dir = Path(run_dir)
    cases = collect(run_dir)
    sheets = {}
    for c in cases:
        slug = c['dir'].split('/')[0]
        if slug not in sheets:
            sheets[slug] = sheet(run_dir, c)
        c['sheet'] = sheets[slug]
    (run_dir / 'results.json').write_text(json.dumps(dict(meta=meta, cases=cases), indent=1))
    (run_dir / 'report.md').write_text(markdown(cases, meta))
    (run_dir / 'index.html').write_text(page(run_dir, cases, meta))
    return run_dir / 'index.html'


def sheet(run_dir, case):
    """A scenario with `Sheet: "1 ship", "6 ships", ...` gets a contact sheet in its
    directory, `sheet.png`: a row per case (resolution), a column per named shot, each
    cell the shot's `-layout.png` copy when it has one (the HUD's panels drawn on it),
    labelled with the case, the shot and the case's result. Rebuilt with each report,
    so it fills in as the cases finish. Returns its path relative to the run, or None."""
    from . import scenario
    try:
        names = scenario.parse(config_repo() / case['file']).sheet
    except Exception:
        return None
    if not names:
        return None
    import cv2
    import numpy as np
    run_dir = Path(run_dir)
    slug_dir = run_dir / case['dir'].split('/')[0]
    rows = []
    for cd in sorted(p.parent for p in slug_dir.glob('*/result.json')):
        r = json.loads((cd / 'result.json').read_text())
        shots = {}
        for line in (cd / 'steps.jsonl').read_text().splitlines() if (cd / 'steps.jsonl').exists() else []:
            rec = json.loads(line)
            if rec.get('kind') == 'shot' and rec.get('shots'):
                shots[rec['text']] = cd / rec['shots'][0]
        rows.append((r, shots))
    rows.sort(key=lambda rs: tuple(int(v) for v in rs[0]['resolution'].split('x')))
    CH, CW, LH = 360, 840, 56          # a cell fits 21:9 at 360 px tall; the label above it
    sheet_img = np.full((len(rows) * (CH + LH) + 8, len(names) * (CW + 8) + 8, 3), 24, np.uint8)
    for i, (r, shots) in enumerate(rows):
        for j, name in enumerate(names):
            x0, y0 = 8 + j * (CW + 8), i * (CH + LH) + 8
            p = shots.get(name + ' (layout)') or shots.get(name)
            img = cv2.imread(str(p)) if p and p.exists() else None
            # Which of this shot's checks failed: the steps between it and the next shot.
            verdict = _verdict_after(r, name, names)
            colour = {'pass': (90, 200, 120), 'fail': (90, 90, 255)}.get(verdict, (200, 200, 200))
            label = f"{r['resolution']} ({r['aspect']})  {name}  {verdict.upper() if verdict else ''}"
            cv2.putText(sheet_img, label, (x0, y0 + 36), cv2.FONT_HERSHEY_SIMPLEX, 0.9, colour, 2, cv2.LINE_AA)
            if img is None:
                cv2.putText(sheet_img, 'no shot', (x0 + 20, y0 + LH + CH // 2), cv2.FONT_HERSHEY_SIMPLEX, 1.0,
                            (150, 150, 150), 2, cv2.LINE_AA)
                continue
            h, w = img.shape[:2]
            k = min(CH / h, CW / w)
            img = cv2.resize(img, (max(1, int(w * k)), max(1, int(h * k))), interpolation=cv2.INTER_AREA)
            sheet_img[y0 + LH:y0 + LH + img.shape[0], x0:x0 + img.shape[1]] = img
    out = slug_dir / 'sheet.png'
    cv2.imwrite(str(out), sheet_img)
    return str(out.relative_to(run_dir))


def _verdict_after(r, name, names):
    """pass or fail for the checks that follow the shot called `name` in a case, up to
    the next shot of the sheet; '' when none ran."""
    texts = [s['text'] for s in r['steps']]
    start = next((k for k, t in enumerate(texts) if f'"{name}"' in t), None)
    if start is None:
        return ''
    out = ''
    for s in r['steps'][start + 1:]:
        if any(f'"{n}"' in s['text'] for n in names):
            break
        if s['status'] in ('fail', 'error'):
            return 'fail'
        if s['status'] == 'pass':
            out = 'pass'
    return out


def config_repo():
    from . import config
    return config.REPO


def failures(cases):
    out = []
    for c in cases:
        for s in c['steps']:
            if s['status'] in ('fail', 'error'):
                out.append((c, s))
    return out


def markdown(cases, meta):
    counts = summary_counts(cases)
    L = [f'# Test run {meta["run"]}', '',
         f'{meta.get("started", "")} · {len(cases)} case(s) · ' +
         ', '.join(f'{v} {k}' for k, v in sorted(counts.items(), key=lambda kv: STATUS_ORDER.index(kv[0]))), '']
    fails = failures(cases)
    if fails:
        L += ['## What failed', '']
        for c, s in fails:
            first = (s.get('detail') or '').strip().splitlines()[0:1]
            L.append(f'- **{c["scenario"]}** at {c["resolution"]} ({c["aspect"]}), step {s["n"]}: '
                     f'{s["text"]} — {first[0] if first else ""}')
        L.append('')
    reviews = [(c, s) for c in cases for s in c['steps'] if s['status'] in ('review', 'inconclusive')]
    if reviews:
        L += ['## Needs a person to look', '']
        for c, s in reviews:
            L.append(f'- **{c["scenario"]}** at {c["resolution"]}, step {s["n"]}: {s["text"]} '
                     f'({", ".join(s.get("shots", []))})')
        L.append('')
    for sh in sorted({c['sheet'] for c in cases if c.get('sheet')}):
        L += [f'Contact sheet: `{sh}`', '']
    L += ['## Cases', '', '| scenario | resolution | aspect | mod | result | time |', '|---|---|---|---|---|---|']
    for c in cases:
        L.append(f'| {c["scenario"]} | {c["resolution"]} | {c["aspect"]} | {c["mod"]} | '
                 f'{BADGE[c["status"]]} | {c["duration"]:.0f}s |')
    L.append('')
    for c in cases:
        L += [f'### {c["scenario"]} — {c["resolution"]} ({c["aspect"]}): {BADGE[c["status"]]}', '']
        for s in c['steps']:
            L.append(f'{s["n"] if s["n"] is not None else "·"}. [{BADGE.get(s["status"], s["status"])}] {s["text"]}')
            if s.get('detail') and s['status'] != 'ok':
                L.append(f'   - {s["detail"].strip().splitlines()[0]}')
            elif s.get('detail'):
                L.append(f'   - {s["detail"].strip().splitlines()[0]}')
            if s.get('how'):
                L.append(f'   - judged by: {s["how"]}')
        L += ['', f'Logs: `{c["dir"]}/log.txt`, `{c["dir"]}/logs/`', '']
    return '\n'.join(L)


def _esc(s):
    return html.escape(str(s or ''))


def page(run_dir, cases, meta):
    counts = summary_counts(cases)
    fails = failures(cases)
    parts = [f"""<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>A2 test run {_esc(meta['run'])}</title>
<style>
:root {{ --bg:#fbfbfa; --fg:#1d1d1b; --muted:#6b6b66; --line:#e3e2dd; --card:#fff;
  --pass:#1f7a3a; --fail:#b3261e; --review:#8a5a00; --skip:#8a8a85; }}
@media (prefers-color-scheme: dark) {{ :root {{ --bg:#141413; --fg:#ecebe6; --muted:#9c9b95;
  --line:#2c2c2a; --card:#1c1c1b; --pass:#5cc47a; --fail:#ff8a80; --review:#f0b94a; --skip:#77776f; }} }}
body {{ background:var(--bg); color:var(--fg); font:15px/1.5 system-ui, sans-serif; margin:0; padding:24px 16px 64px; }}
main {{ max-width:1200px; margin:0 auto; }}
h1 {{ font-size:22px; margin:0 0 4px; }} h2 {{ font-size:18px; margin:36px 0 12px; }}
h3 {{ font-size:16px; margin:0; }}
.muted {{ color:var(--muted); }}
.b {{ display:inline-block; font:600 12px/1 ui-monospace, monospace; padding:4px 7px; border-radius:4px;
  border:1px solid currentColor; }}
.pass,.ok {{ color:var(--pass); }} .fail,.error {{ color:var(--fail); }}
.review,.inconclusive {{ color:var(--review); }} .skip {{ color:var(--skip); }}
table {{ border-collapse:collapse; width:100%; }} td,th {{ text-align:left; padding:6px 8px; border-bottom:1px solid var(--line); vertical-align:top; }}
.case {{ background:var(--card); border:1px solid var(--line); border-radius:8px; padding:16px; margin:16px 0; }}
.step {{ padding:8px 0; border-top:1px solid var(--line); }}
.step .d {{ white-space:pre-wrap; font:13px/1.45 ui-monospace, monospace; color:var(--muted); margin:4px 0 0; }}
.shots {{ display:flex; gap:8px; flex-wrap:wrap; margin-top:6px; }}
.shots a img {{ height:150px; border:1px solid var(--line); border-radius:4px; display:block; }}
.shots figure {{ margin:0; }} .shots figcaption {{ font-size:12px; color:var(--muted); }}
.gallery {{ display:grid; grid-template-columns:repeat(auto-fill, minmax(260px, 1fr)); gap:12px; }}
.gallery img {{ width:100%; border:1px solid var(--line); border-radius:4px; }}
details > summary {{ cursor:pointer; }}
code {{ font:13px ui-monospace, monospace; }}
</style></head><body><main>
<h1>Armada II test run <code>{_esc(meta['run'])}</code></h1>
<div class="muted">{_esc(meta.get('started'))} · {len(cases)} case(s) · {_esc(meta.get('command', ''))}</div>
<p>""" + ' '.join(f'<span class="b {k}">{v} {BADGE[k]}</span>' for k, v in
                   sorted(counts.items(), key=lambda kv: STATUS_ORDER.index(kv[0]))) + '</p>']

    sheets = sorted({c['sheet'] for c in cases if c.get('sheet')})
    if sheets:
        parts.append('<h2>Contact sheets</h2>' + ''.join(
            f'<p><a href="{_esc(sh)}"><img src="{_esc(sh)}" alt="contact sheet {_esc(sh)}" '
            f'style="max-width:100%;border:1px solid var(--line);border-radius:4px"></a></p>' for sh in sheets))
    if fails:
        parts.append('<h2>What failed</h2><ul>')
        for c, s in fails:
            first = (s.get('detail') or '').strip().splitlines()[:1]
            parts.append(f'<li><a href="#{_anchor(c)}">{_esc(c["scenario"])} at {_esc(c["resolution"])}</a>, '
                         f'step {s["n"]}: {_esc(s["text"])} — <span class="fail">{_esc(first[0] if first else "")}</span></li>')
        parts.append('</ul>')

    parts.append('<h2>Cases</h2><table><tr><th>scenario</th><th>resolution</th><th>aspect</th><th>mod</th>'
                 '<th>result</th><th>time</th></tr>')
    for c in cases:
        parts.append(f'<tr><td><a href="#{_anchor(c)}">{_esc(c["scenario"])}</a></td><td>{c["resolution"]}</td>'
                     f'<td>{c["aspect"]}</td><td>{c["mod"]}</td><td><span class="b {c["status"]}">'
                     f'{BADGE[c["status"]]}</span></td><td>{c["duration"]:.0f}s</td></tr>')
    parts.append('</table>')

    # screenshots side by side across resolutions, per scenario and shot name
    by_scn = {}
    for c in cases:
        by_scn.setdefault(c['scenario'], []).append(c)
    gal = []
    for scn, cs in by_scn.items():
        names = {}
        for c in cs:
            for p in sorted((run_dir / c['dir'] / 'shots').glob('*.png')):
                n = re.sub(r'^\d+-', '', p.stem)
                if n.startswith(('check-', 'before-click', 'timeout-')) or n.endswith(('-found', '-text')):
                    continue
                names.setdefault(n, []).append((c, p))
        for n, items in names.items():
            if len({(c['resolution'], c['mod']) for c, _ in items}) < 2:
                continue
            gal.append(f'<h3>{_esc(scn)} — “{_esc(n)}”</h3><div class="gallery">')
            for c, p in items:
                rel = p.relative_to(run_dir)
                gal.append(f'<figure style="margin:0"><a href="{_esc(rel)}"><img loading="lazy" src="{_esc(rel)}"></a>'
                           f'<figcaption class="muted">{c["resolution"]} · {c["aspect"]} · {c["mod"]}</figcaption></figure>')
            gal.append('</div>')
    if gal:
        parts.append('<h2>Compare across resolutions</h2>' + '\n'.join(gal))

    parts.append('<h2>Every step</h2>')
    for c in cases:
        cd = run_dir / c['dir']
        parts.append(f'<section class="case" id="{_anchor(c)}"><h3>{_esc(c["scenario"])} — {c["resolution"]} '
                     f'({c["aspect"]}, {c["mod"]}) <span class="b {c["status"]}">{BADGE[c["status"]]}</span></h3>'
                     f'<div class="muted">{_esc(c["file"])} · {c["duration"]:.0f}s · '
                     f'<a href="{_esc(c["dir"])}/log.txt">detailed log</a> · '
                     f'<a href="{_esc(c["dir"])}/logs/">logs</a>' +
                     (f' · <a href="{_esc(c["dir"])}/run.mp4">video</a>' if (cd / 'run.mp4').exists() else '') +
                     '</div>')
        for s in c['steps']:
            parts.append(f'<div class="step"><span class="b {s["status"]}">{BADGE.get(s["status"], s["status"])}</span> '
                         f'{s["n"] if s["n"] is not None else "·"}. {_esc(s["text"])}')
            if s.get('detail'):
                parts.append(f'<div class="d">{_esc(s["detail"])}</div>')
            if s.get('how'):
                parts.append(f'<div class="d">judged by: {_esc(s["how"])}</div>')
            shots = s.get('shots') or []
            if shots:
                parts.append('<div class="shots">')
                for sh in shots:
                    p = Path(sh)
                    rel = (Path(c['dir']) / sh) if not p.is_absolute() else _rel_to(p, run_dir)
                    parts.append(f'<figure><a href="{_esc(rel)}"><img loading="lazy" src="{_esc(rel)}"></a>'
                                 f'<figcaption>{_esc(p.name)}</figcaption></figure>')
                parts.append('</div>')
            parts.append('</div>')
        # a case with `Players:` keeps each game's logs and shots under <case>/<player>/
        logs = sorted(cd.glob('logs/*')) + sorted(cd.glob('*/logs/*'))
        if logs:
            parts.append('<details><summary class="muted">logs from this case</summary><ul>' +
                         ''.join(f'<li><a href="{_esc(c["dir"])}/{_esc(l.relative_to(cd))}">'
                                 f'{_esc(l.relative_to(cd))}</a> '
                                 f'<span class="muted">{l.stat().st_size if l.is_file() else ""} B</span></li>'
                                 for l in logs) + '</ul></details>')
        allshots = sorted(cd.glob('shots/*.png')) + sorted(cd.glob('*/shots/*.png'))
        if allshots:
            parts.append(f'<details><summary class="muted">all {len(allshots)} screenshots</summary><div class="shots">' +
                         ''.join(f'<figure><a href="{_esc(c["dir"])}/{_esc(p.relative_to(cd))}"><img loading="lazy" '
                                 f'src="{_esc(c["dir"])}/{_esc(p.relative_to(cd))}"></a><figcaption>'
                                 f'{_esc(p.relative_to(cd))}</figcaption></figure>' for p in allshots) +
                         '</div></details>')
        parts.append('</section>')
    parts.append('</main></body></html>')
    return '\n'.join(parts)


def _anchor(c):
    return re.sub(r'[^a-z0-9]+', '-', f'{c["dir"]}'.lower())


def _rel_to(p, base):
    try:
        return p.resolve().relative_to(Path(base).resolve())
    except ValueError:
        return p
