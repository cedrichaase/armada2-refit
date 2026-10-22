#!/usr/bin/env python3
"""Prepare false-positive reports for a release from its VirusTotal scan.

    publish/virustotal-fp.py --report virustotal.json --tag vX.Y.Z --repo owner/name
        [--commit <sha>] [--files <dir>] [--samples <dir>] [--artifact-url <url>]
        [--vendors publish/av-vendors.json] [--body <file>] [--issue] [--vt-comment]
    publish/virustotal-fp.py --selftest

Reads the virustotal.json that publish/virustotal.py wrote for the release, groups the
flagged files by antivirus vendor (publish/av-vendors.json, kept by hand) and writes one
issue body: a checklist item per vendor, with where to submit, the files with their
hashes, VirusTotal links and detection names, and a submission text to paste. With
--issue it opens the issue "False positives: <tag>", or updates it, keeping the items
already ticked. With --files it copies each flagged file (found by hash in the release
zip or beside it) into --samples, for the workflow to zip with the password vendors
expect. With --vt-comment it posts a comment on each flagged file on VirusTotal (the
source, and that CI built it at the commit) and a harmless vote, once per commit:
$VT_API_KEY. Run by .github/workflows/virustotal-fp.yml; publish/README.md, "VirusTotal".
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import zipfile

import importlib.util

_spec = importlib.util.spec_from_file_location(
    'virustotal', os.path.join(os.path.dirname(os.path.abspath(__file__)), 'virustotal.py'))
vt = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(vt)

SERVER = os.environ.get('GITHUB_SERVER_URL', 'https://github.com')


def flagged(report):
    return [e for e in report['files']
            if e.get('status') == 'done' and (e.get('malicious') or e.get('suspicious'))]


def group(files, vendors):
    """{vendor name: {'vendor': entry, 'hits': [(file, detection)]}}, in vendors' order
    first, then engines nobody lists, alphabetically."""
    by_engine = {eng: v for v in vendors for eng in v['engines']}
    groups = {}
    for f in files:
        for d in f.get('detections', []):
            v = by_engine.get(d['engine']) or {
                'vendor': d['engine'], 'engines': [d['engine']], 'url': '', 'email': '',
                'note': 'Not in publish/av-vendors.json: look the address up and add it.'}
            groups.setdefault(v['vendor'], {'vendor': v, 'hits': []})['hits'].append((f, d))
    order = {v['vendor']: i for i, v in enumerate(vendors)}
    return dict(sorted(groups.items(), key=lambda kv: (order.get(kv[0], len(order)),
                                                       kv[0].lower())))


def contact(v):
    parts = []
    if v.get('url'):
        parts.append(v['url'])
    if v.get('email'):
        parts.append(f'<{v["email"]}>')
    return ' or '.join(parts) or 'no contact on file'


def submission(ctx, vendor, hits):
    names = sorted({d['result'] or d['category'] for _, d in hits})
    lines = [
        f'Subject: False positive: {", ".join(names)} in Armada II Refit {ctx["version"]}',
        '',
        'Hello,',
        '',
        f'{vendor} detects the files below from Armada II Refit {ctx["version"]}. We '
        'believe these are false positives and ask you to review them.',
        '',
        'Armada II Refit is an open-source (MIT) fan project for the 2001 PC game Star Trek: '
        'Armada II. Its plugins are DLLs (most named .asi) that the game loads '
        'into its own process, where they change how the game draws and plays; hooking '
        'that one process is what they are for. They are not packed or obfuscated, and each '
        'carries a version resource naming the project.',
    ]
    paths = {f['path'] for f, _ in hits}
    if any(p.lower().endswith('.exe') for p in paths):
        lines.append('The .exe is the project\'s installer: a Python script from the same '
                     'repository (gui-installer/), frozen with PyInstaller.')
    if any(p.startswith(('vendor/', 'chain/')) for p in paths):
        lines.append('The files under vendor/ and chain/ are third-party open-source '
                     'releases, bundled unmodified.')
    lines += [
        '',
        f'Source, at the released commit: {ctx["tree"]}',
        f'Our files are built from that source in public CI (GitHub Actions), build log: '
        f'{ctx["run_url"] or "(not recorded)"}',
        f'The release: {ctx["release"]}',
        '',
        'Files:',
    ]
    seen = set()
    for f, d in hits:
        key = (f['sha256'], d['engine'])
        if key in seen:
            continue
        seen.add(key)
        lines.append(f'- {f["path"]}  SHA-256 {f["sha256"]}  detected as '
                     f'"{d["result"] or d["category"]}" ({d["engine"]})  {f["link"]}')
    lines += ['', 'The files are attached as a zip, password "infected".', '',
              'Thank you,', f'the Armada II Refit project ({SERVER}/{ctx["repo"]})']
    return '\n'.join(lines)


def body(report, groups, ctx, ticked=()):
    files = flagged(report)
    out = [
        f'VirusTotal scan of [{ctx["tag"]}]({ctx["release"]}) ({report.get("scanned")}, '
        f'[build and scan log]({ctx["run_url"]})): {len(files)} of {len(report["files"])} '
        f'files flagged by {len(groups)} vendors. Source at `{ctx["commit"]}`: '
        f'{ctx["tree"]}.',
        '',
        'Tick a vendor once its submission is sent. The flagged files, zipped with the '
        'password `infected`, are '
        + (f'[an artifact of this run]({ctx["artifact_url"]})' if ctx.get('artifact_url')
           else 'an artifact of the `virustotal-fp` run')
        + ' (kept 90 days; re-run the workflow for a fresh one). Written by '
        '`.github/workflows/virustotal-fp.yml`; addresses from `publish/av-vendors.json`, '
        'kept by hand: correct it when one is wrong.',
        '',
        '## Vendors',
        '',
    ]
    for name, g in groups.items():
        v, hits = g['vendor'], g['hits']
        box = 'x' if name in ticked else ' '
        engines = ', '.join(sorted({d['engine'] for _, d in hits}))
        out.append(f'- [{box}] **{name}** ({engines}): {contact(v)}')
        if v.get('note'):
            out.append(f'  {v["note"]}')
        for f, d in hits:
            out.append(f'  - `{f["path"]}`: {d["result"] or d["category"]}'
                       f'{"" if d["category"] == "malicious" else " (suspicious)"}, '
                       f'[VirusTotal]({f["link"]})')
        out += ['  <details><summary>Submission text</summary>', '',
                '  ```', *('  ' + l if l else '' for l in submission(ctx, name, hits).split('\n')),
                '  ```', '  </details>', '']
    out += ['## Flagged files', '', '| File | SHA-256 | Detections | Report |', '|---|---|---|---|']
    for f in files:
        out.append(f'| `{f["path"]}` | `{f["sha256"]}` | {vt.ratio(f)} | '
                   f'[VirusTotal]({f["link"]}) |')
    if not files:
        out.append('| none | | | |')
    return '\n'.join(out) + '\n'


TICK = re.compile(r'^- \[[xX]\] \*\*(.+?)\*\*', re.M)


def ticked_in(old_body):
    return set(TICK.findall(old_body or ''))


def samples(files, src_dir, dest):
    """Copy each flagged file, found by hash in src_dir's zips or as a file there."""
    want = {f['sha256']: f for f in files}
    os.makedirs(dest, exist_ok=True)
    found = {}
    for name in sorted(os.listdir(src_dir)):
        p = os.path.join(src_dir, name)
        if name.endswith('.zip'):
            with zipfile.ZipFile(p) as z:
                for info in z.infolist():
                    if info.is_dir():
                        continue
                    data = z.read(info)
                    h = hashlib.sha256(data).hexdigest()
                    if h in want and h not in found:
                        found[h] = data
        elif os.path.isfile(p):
            with open(p, 'rb') as fh:
                data = fh.read()
            h = hashlib.sha256(data).hexdigest()
            if h in want and h not in found:
                found[h] = data
    for h, data in found.items():
        f = want[h]
        # The path keeps two files of one name apart (game/x.dll, chain/x.dll).
        out = os.path.join(dest, f['path'].replace('/', '__'))
        with open(out, 'wb') as fh:
            fh.write(data)
    missing = [want[h]['path'] for h in want if h not in found]
    return len(found), missing


def gh(*args, input=None):
    r = subprocess.run(['gh', *args], input=input, capture_output=True, text=True)
    if r.returncode:
        raise SystemExit(f'gh {" ".join(args)}: {r.stderr.strip()}')
    return r.stdout


def find_issue(repo, title):
    out = gh('issue', 'list', '--repo', repo, '--state', 'all', '--limit', '100',
             '--search', f'in:title "{title}"', '--json', 'number,title,body')
    for i in json.loads(out or '[]'):
        if i['title'] == title:
            return i
    return None


def comment_on_vt(client, files, ctx):
    marker = f'built from source by CI at {ctx["commit"]}'
    for f in files:
        try:
            existing = client.request('GET', f'/files/{f["sha256"]}/comments?limit=40')
            if any(marker in (c.get('attributes', {}).get('text') or '')
                   for c in existing.get('data', [])):
                print(f'{f["path"]}: already commented for {ctx["commit"][:12]}')
                continue
            client.comment(f['sha256'],
                           f'Armada II Refit {ctx["version"]} ({f["path"]}), an open-source '
                           f'(MIT) plugin for Star Trek: Armada II, {marker}: source '
                           f'{ctx["tree"]}, build log {ctx["run_url"]}. Detections of it are '
                           'false positives; we are reporting them to the vendors.')
            client.vote(f['sha256'], 'harmless')
            print(f'{f["path"]}: commented and voted harmless')
        except vt.ApiError as e:
            print(f'::warning::{f["path"]}: VirusTotal comment failed: {e}')


def context(report, a):
    commit = a.commit or report.get('commit') or a.tag
    return {'tag': a.tag, 'repo': a.repo, 'commit': commit,
            'version': report.get('version') or a.tag.lstrip('v'),
            'tree': f'{SERVER}/{a.repo}/tree/{commit}',
            'release': f'{SERVER}/{a.repo}/releases/tag/{a.tag}',
            'run_url': report.get('run_url') or '',
            'artifact_url': a.artifact_url}


def run(a):
    with open(a.report) as f:
        report = json.load(f)
    with open(a.vendors) as f:
        vendors = json.load(f)['vendors']
    files = flagged(report)
    groups = group(files, vendors)
    ctx = context(report, a)
    print(f'{len(files)} flagged files, {len(groups)} vendors')
    if a.files and a.samples:
        n, missing = samples(files, a.files, a.samples)
        print(f'{n} samples in {a.samples}')
        for m in missing:
            print(f'::warning::flagged file not found in {a.files}: {m}')
    title = f'False positives: {a.tag}'
    existing = find_issue(a.repo, title) if a.issue else None
    text = body(report, groups, ctx, ticked_in(existing['body']) if existing else ())
    if a.body:
        with open(a.body, 'w') as f:
            f.write(text)
    if a.issue:
        if not files:
            print('nothing flagged: no issue')
        else:
            with tempfile.NamedTemporaryFile('w', suffix='.md', delete=False) as f:
                f.write(text)
            if existing:
                gh('issue', 'edit', str(existing['number']), '--repo', a.repo,
                   '--body-file', f.name)
                print(f'updated {SERVER}/{a.repo}/issues/{existing["number"]}')
            else:
                print(gh('issue', 'create', '--repo', a.repo, '--title', title,
                         '--body-file', f.name).strip())
    elif not a.body:
        print(text)
    if a.vt_comment and files:
        key = os.environ.get('VT_API_KEY', '').strip()
        if not key:
            print('::notice::VirusTotal comments skipped: no VT_API_KEY secret')
        else:
            comment_on_vt(vt.Client(key, float(os.environ.get('VT_INTERVAL', '15.5'))),
                          files, ctx)
    return 0


def selftest():
    tmp = tempfile.mkdtemp()
    mz = lambda tag: b'MZ\x90\x00' + tag.encode()
    sha = lambda b: hashlib.sha256(b).hexdigest()
    entry = lambda path, data, dets: {
        'path': path, 'name': os.path.basename(path), 'sha256': sha(data), 'size': len(data),
        'link': vt.GUI + sha(data), 'status': 'done', 'source': 'known',
        'malicious': sum(d[1] == 'malicious' for d in dets),
        'suspicious': sum(d[1] == 'suspicious' for d in dets), 'total': 70,
        'detections': [{'engine': e, 'category': c, 'result': r} for e, c, r in dets]}
    report = {'schema': 1, 'version': '9.9.9', 'commit': 'f' * 40, 'scanned': 'now',
              'run_url': 'https://ci.example/run/1', 'files': [
                  entry('game/Menus.asi', mz('menus'), [
                      ('BitDefender', 'malicious', 'Gen:Variant.Zusy.1'),
                      ('MicroWorld-eScan', 'malicious', 'Gen:Variant.Zusy.1'),
                      ('Obscure-AV', 'suspicious', 'Heur.X')]),
                  entry('game/QOL.asi', mz('qol'), [('BitDefender', 'malicious', 'Gen:Variant.Zusy.2')]),
                  entry('game/UI.asi', mz('ui'), []),
                  dict(entry('game/Late.asi', mz('late'), []), status='pending')]}
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'av-vendors.json')) as f:
        vendors = json.load(f)['vendors']
    files = flagged(report)
    assert [f['path'] for f in files] == ['game/Menus.asi', 'game/QOL.asi'], files
    groups = group(files, vendors)
    assert list(groups) == ['Bitdefender', 'eScan (MicroWorld)', 'Obscure-AV'], list(groups)
    assert len(groups['Bitdefender']['hits']) == 2
    a = argparse.Namespace(tag='v9.9.9', repo='o/r', commit=None, artifact_url='https://a/1')
    ctx = context(report, a)
    text = body(report, groups, ctx)
    assert '- [ ] **Bitdefender** (BitDefender): <virus_submission@bitdefender.com>' in text, text
    assert '- [ ] **Obscure-AV** (Obscure-AV): no contact on file' in text, text
    assert f'{SERVER}/o/r/tree/{"f" * 40}' in text and 'https://ci.example/run/1' in text
    assert 'Subject: False positive: Gen:Variant.Zusy.1, Gen:Variant.Zusy.2 in Armada II Refit 9.9.9' in text, text
    assert sha(mz('qol')) in text and 'game/UI.asi' not in text and 'Late.asi' not in text
    assert '[an artifact of this run](https://a/1)' in text
    # An update keeps what was ticked.
    old = text.replace('- [ ] **Bitdefender**', '- [x] **Bitdefender**')
    again = body(report, groups, ctx, ticked_in(old))
    assert '- [x] **Bitdefender**' in again and '- [ ] **Obscure-AV**' in again
    # Samples come out of the zip by hash, and a missing one is named.
    src = os.path.join(tmp, 'dl')
    os.makedirs(src)
    with zipfile.ZipFile(os.path.join(src, 'armada2-refit-9.9.9.zip'), 'w') as z:
        z.writestr('armada2-refit-9.9.9/game/Menus.asi', mz('menus'))
        z.writestr('armada2-refit-9.9.9/game/UI.asi', mz('ui'))
    n, missing = samples(files, src, os.path.join(tmp, 'samples'))
    assert n == 1 and missing == ['game/QOL.asi'], (n, missing)
    assert open(os.path.join(tmp, 'samples', 'game__Menus.asi'), 'rb').read() == mz('menus')
    # VirusTotal comments: once per commit.
    fake = vt.FakeVT({})
    fake.throttled = True
    posted = []

    def send(req, _orig=fake._send):
        m, url = req.get_method(), req.full_url
        if url.endswith('/comments?limit=40'):
            done = sha(mz('qol')) in url
            text = f'... built from source by CI at {"f" * 40} ...' if done else 'other'
            return 200, json.dumps({'data': [{'attributes': {'text': text}}]}).encode()
        if m == 'POST':
            posted.append((url.rsplit('/', 1)[1], json.loads(req.data)))
            return 200, b'{"data": {}}'
        return _orig(req)
    fake._send = send
    import contextlib
    import io
    with contextlib.redirect_stdout(io.StringIO()):
        comment_on_vt(fake, files, ctx)
    assert [p[0] for p in posted] == ['comments', 'votes'], posted
    assert posted[1][1]['data']['attributes']['verdict'] == 'harmless'
    print('selftest: ok', file=sys.stderr)
    return 0


def main():
    if sys.argv[1:] == ['--selftest']:
        return selftest()
    here = os.path.dirname(os.path.abspath(__file__))
    p = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    p.add_argument('--report', required=True)
    p.add_argument('--tag', required=True)
    p.add_argument('--repo', required=True)
    p.add_argument('--commit', help="the tag's full commit (default: the report's)")
    p.add_argument('--vendors', default=os.path.join(here, 'av-vendors.json'))
    p.add_argument('--files', help='folder with the release zip and installer')
    p.add_argument('--samples', help='folder to copy the flagged files into')
    p.add_argument('--artifact-url', help='where the samples zip is, for the issue')
    p.add_argument('--body', help='write the issue body here')
    p.add_argument('--issue', action='store_true', help='open or update the issue (gh)')
    p.add_argument('--vt-comment', action='store_true',
                   help='comment on and vote harmless for each flagged file on VirusTotal')
    return run(p.parse_args())


if __name__ == '__main__':
    sys.exit(main())
