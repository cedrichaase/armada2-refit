#!/usr/bin/env python3
"""Scan a release's programs on VirusTotal and write what the engines say.

    publish/virustotal.py scan --zip <release.zip> [--extra <file>]... [--out <dir>]
                               [--commit <sha>] [--run-url <url>] [--timeout <s>]
    publish/virustotal.py notes <virustotal.json> --assets <url>   (the release-notes block)
    publish/virustotal.py --selftest

`scan` takes every PE in the release zip (the members ending .asi, .dll or .exe that start
with "MZ": the plugins in game/, Display8 in display/, the chain in chain/, the
prerequisites in vendor/), and
each --extra file (CI passes the Windows GUI installer). Nothing is named here, so a
plugin that is added, merged or removed is followed without an edit. For each one it
looks the hash up first (GET /files/{sha256}: a known file costs no upload, which covers
the vendored DLLs), and uploads the rest (POST /files, or an upload URL above 32 MB) and
polls their analyses until they complete or --timeout runs out. It writes
virustotal.json (read by publish/virustotal-fp.py) and virustotal.md (a table), and
appends the table to $GITHUB_STEP_SUMMARY when that is set.

VirusTotal API v3, standard library only. The key is $VT_API_KEY; without one the scan
is skipped with a notice and exits 0 (forks, pull requests from forks). The public API
allows 4 requests a minute: requests are spaced $VT_INTERVAL seconds apart (default 15.5)
and an HTTP 429 waits a minute and tries again. A file still being analysed at the
timeout is reported as pending, with its link, and does not fail the scan.

$VT_FAIL_ABOVE (unset or empty by default) turns the report into a gate: the scan exits 3
when any file has more "malicious" verdicts than that number. The reports are written
first either way. publish/README.md, "VirusTotal".
"""
import argparse
import hashlib
import io
import json
import os
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
import zipfile

API = 'https://www.virustotal.com/api/v3'
GUI = 'https://www.virustotal.com/gui/file/'
BIG = 32 * 1024 * 1024           # above this, POST /files refuses: ask for an upload URL
PE_EXT = ('.asi', '.dll', '.exe')
COUNTED = ('malicious', 'suspicious', 'undetected', 'harmless')   # VirusTotal's x/y


def log(msg):
    print(msg, file=sys.stderr, flush=True)


class ApiError(Exception):
    def __init__(self, status, body):
        super().__init__(f'HTTP {status}: {body[:300]}')
        self.status = status


class Client:
    """VirusTotal API v3 over urllib, spaced to the public rate limit."""

    def __init__(self, key, interval=15.5, sleep=time.sleep, clock=time.monotonic):
        self.key, self.interval, self.sleep, self.clock = key, interval, sleep, clock
        self.last = None
        self.calls = 0

    def _wait(self):
        if self.last is not None:
            gap = self.interval - (self.clock() - self.last)
            if gap > 0:
                self.sleep(gap)
        self.last = self.clock()

    def _send(self, req):
        """One HTTP exchange: (status, body bytes). Overridden by the self-test."""
        try:
            with urllib.request.urlopen(req, timeout=300) as r:
                return r.status, r.read()
        except urllib.error.HTTPError as e:
            return e.code, e.read()

    def request(self, method, url, body=None, headers=None):
        if url.startswith('/'):
            url = API + url
        h = {'x-apikey': self.key, 'accept': 'application/json'}
        h.update(headers or {})
        for attempt in range(6):
            self._wait()
            self.calls += 1
            status, data = self._send(urllib.request.Request(url, data=body, headers=h,
                                                             method=method))
            if status == 429 or status >= 500:
                log(f'  {method} {url}: HTTP {status}, waiting a minute ({attempt + 1}/6)')
                self.sleep(60)
                continue
            if status >= 400:
                raise ApiError(status, data.decode('utf-8', 'replace'))
            return json.loads(data or b'{}')
        raise ApiError(status, 'gave up after repeated rate limiting or server errors')

    def get_file(self, sha256):
        try:
            return self.request('GET', f'/files/{sha256}')['data']['attributes']
        except ApiError as e:
            if e.status == 404:
                return None
            raise

    def upload(self, name, data):
        url = '/files'
        if len(data) > BIG:
            url = self.request('GET', '/files/upload_url')['data']
        boundary = uuid.uuid4().hex
        body = b''.join([
            f'--{boundary}\r\n'.encode(),
            f'Content-Disposition: form-data; name="file"; filename="{name}"\r\n'.encode(),
            b'Content-Type: application/octet-stream\r\n\r\n', data,
            f'\r\n--{boundary}--\r\n'.encode()])
        r = self.request('POST', url, body,
                         {'content-type': f'multipart/form-data; boundary={boundary}'})
        return r['data']['id']

    def analysis(self, analysis_id):
        return self.request('GET', f'/analyses/{analysis_id}')['data']['attributes']

    def comment(self, sha256, text):
        body = json.dumps({'data': {'type': 'comment', 'attributes': {'text': text}}})
        return self.request('POST', f'/files/{sha256}/comments', body.encode(),
                            {'content-type': 'application/json'})

    def vote(self, sha256, verdict='harmless'):
        body = json.dumps({'data': {'type': 'vote', 'attributes': {'verdict': verdict}}})
        return self.request('POST', f'/files/{sha256}/votes', body.encode(),
                            {'content-type': 'application/json'})


def is_pe(data):
    return data[:2] == b'MZ'


def collect(zip_path, extras):
    """Every PE in the zip, by extension and MZ header, then the extra files.
    -> [(path shown, file name, bytes)]. The zip's top folder is dropped from the path."""
    out = []
    meta = {}
    with zipfile.ZipFile(zip_path) as z:
        for info in z.infolist():
            parts = info.filename.split('/', 1)
            rel = parts[1] if len(parts) == 2 else parts[0]
            if rel == 'manifest.json':
                try:
                    meta = json.loads(z.read(info))
                except ValueError:
                    pass
            if info.is_dir() or not rel.lower().endswith(PE_EXT):
                continue
            data = z.read(info)
            if is_pe(data):
                out.append((rel, os.path.basename(rel), data))
    for f in extras:
        with open(f, 'rb') as fh:
            data = fh.read()
        if not is_pe(data):
            raise SystemExit(f'not a PE: {f}')
        out.append((os.path.basename(f), os.path.basename(f), data))
    out.sort(key=lambda t: t[0].lower())
    return out, meta


def summarise(entry, stats, results, date=None):
    """Fill an entry's verdict fields from a file's or an analysis's stats and results."""
    results = results or {}
    entry['malicious'] = int(stats.get('malicious', 0))
    entry['suspicious'] = int(stats.get('suspicious', 0))
    entry['total'] = sum(int(stats.get(k, 0)) for k in COUNTED)
    entry['detections'] = sorted(
        ({'engine': eng, 'category': r.get('category'), 'result': r.get('result') or ''}
         for eng, r in results.items() if r.get('category') in ('malicious', 'suspicious')),
        key=lambda d: (d['category'] != 'malicious', d['engine'].lower()))
    if date:
        entry['analysis_date'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(date))
    entry['status'] = 'done'


def scan(client, files, timeout, clock=time.monotonic):
    entries, pending = [], {}
    for path, name, data in files:
        sha = hashlib.sha256(data).hexdigest()
        e = {'path': path, 'name': name, 'sha256': sha, 'size': len(data),
             'link': GUI + sha, 'status': 'pending'}
        entries.append(e)
        log(f'{path}  {sha}')
        try:
            attrs = client.get_file(sha)
            if attrs and attrs.get('last_analysis_results'):
                e['source'] = 'known'
                summarise(e, attrs.get('last_analysis_stats', {}),
                          attrs['last_analysis_results'], attrs.get('last_analysis_date'))
                log(f'  known: {e["malicious"]}/{e["total"]}')
                continue
            if attrs:                     # known, but its first analysis is still running
                e['source'] = 'known'
                pending[sha] = ('file', sha)
                continue
            e['source'] = 'uploaded'
            pending[sha] = ('analysis', client.upload(name, data))
            log('  uploaded')
        except ApiError as err:
            e['status'], e['error'] = 'error', str(err)
            log(f'  {err}')
    deadline = clock() + timeout
    by_sha = {e['sha256']: e for e in entries}
    while pending and clock() < deadline:
        for sha, (kind, ident) in list(pending.items()):
            if clock() >= deadline:
                break
            e = by_sha[sha]
            try:
                if kind == 'analysis':
                    a = client.analysis(ident)
                    if a.get('status') != 'completed':
                        continue
                    summarise(e, a.get('stats', {}), a.get('results'), a.get('date'))
                else:
                    attrs = client.get_file(ident) or {}
                    if not attrs.get('last_analysis_results'):
                        continue
                    summarise(e, attrs.get('last_analysis_stats', {}),
                              attrs['last_analysis_results'], attrs.get('last_analysis_date'))
            except ApiError as err:
                e['status'], e['error'] = 'error', str(err)
            del pending[sha]
            log(f'{e["path"]}: {e["status"]} '
                f'{e.get("malicious", "?")}/{e.get("total", "?")}')
    return entries


def ratio(e):
    if e['status'] == 'done':
        s = f'{e["malicious"]}/{e["total"]}'
        if e['suspicious']:
            s += f' (+{e["suspicious"]} suspicious)'
        return s
    return e['status']


def markdown(report):
    v = report.get('version') or '?'
    head = f'## VirusTotal: Armada II Refit {v}\n\n'
    if report.get('commit'):
        head += f'Commit `{report["commit"]}`, scanned {report["scanned"]}'
    else:
        head += f'Scanned {report["scanned"]}'
    if report.get('run_url'):
        head += f' by [CI]({report["run_url"]})'
    head += ('. Detections are "malicious" verdicts out of the engines that gave one; '
             'every flag on our own files is a false positive to report '
             '(`.github/workflows/virustotal-fp.yml`).\n\n')
    rows = ['| File | SHA-256 | Detections | Flagged by | Report |',
            '|---|---|---|---|---|']
    for e in report['files']:
        flags = '; '.join(f'{d["engine"]}: {d["result"] or d["category"]}'
                          + ('' if d['category'] == 'malicious' else ' (suspicious)')
                          for d in e.get('detections', []))
        if e['status'] == 'error':
            flags = e.get('error', '').replace('|', '/')
        rows.append(f'| `{e["path"]}` | `{e["sha256"]}` | {ratio(e)} | {flags or "none"} '
                    f'| [VirusTotal]({e["link"]}) |')
    flagged = [e for e in report['files'] if e['status'] == 'done'
               and (e['malicious'] or e['suspicious'])]
    tail = (f'\n\n{len(flagged)} of {len(report["files"])} files flagged.\n' if flagged
            else f'\n\nNo engine flags any of the {len(report["files"])} files.\n')
    return head + '\n'.join(rows) + tail


START, END = '<!-- virustotal -->', '<!-- /virustotal -->'


def notes_block(report, assets):
    """The release-notes block: one line, each file linked to its report."""
    parts = [f'[{e["name"]}]({e["link"]}) {ratio(e)}' for e in report['files']]
    return (f'{START}\n**VirusTotal**: every program in this release scanned at '
            f'{report["scanned"]} ([virustotal.md]({assets}/virustotal.md), '
            f'[virustotal.json]({assets}/virustotal.json)): ' + ' · '.join(parts) + f'\n{END}')


def splice_notes(notes, block):
    """Put the block into release notes, replacing an earlier one (a re-scan)."""
    if START in notes and END in notes:
        a = notes.index(START)
        b = notes.index(END) + len(END)
        return notes[:a] + block + notes[b:]
    return notes.rstrip('\n') + '\n\n' + block + '\n'


def fail_above():
    v = os.environ.get('VT_FAIL_ABOVE', '').strip()
    return int(v) if v else None


def write_reports(report, out):
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, 'virustotal.json'), 'w') as f:
        json.dump(report, f, indent=1)
        f.write('\n')
    md = markdown(report)
    with open(os.path.join(out, 'virustotal.md'), 'w') as f:
        f.write(md)
    summary = os.environ.get('GITHUB_STEP_SUMMARY')
    if summary:
        with open(summary, 'a') as f:
            f.write(md + '\n')
    return md


def gate(report):
    limit = fail_above()
    if limit is None:
        return 0
    over = [e for e in report['files'] if e['status'] == 'done' and e['malicious'] > limit]
    for e in over:
        print(f'::error::{e["path"]}: {e["malicious"]} engines flag it '
              f'(VT_FAIL_ABOVE={limit})')
    return 3 if over else 0


def cmd_scan(args, client=None):
    key = os.environ.get('VT_API_KEY', '').strip()
    if client is None and not key:
        print('::notice::VirusTotal scan skipped: no VT_API_KEY secret '
              '(publish/README.md, "VirusTotal")')
        return 0
    files, meta = collect(args.zip, args.extra or [])
    if not files:
        raise SystemExit(f'no PE in {args.zip}')
    client = client or Client(key, float(os.environ.get('VT_INTERVAL', '15.5')))
    log(f'{len(files)} files to scan')
    entries = scan(client, files, args.timeout)
    report = {'schema': 1, 'name': meta.get('name', 'armada2-refit'),
              'version': meta.get('version'), 'commit': args.commit or meta.get('commit'),
              'run_url': args.run_url,
              'scanned': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
              'files': entries}
    print(write_reports(report, args.out))
    log(f'{client.calls} API requests')
    return gate(report)


def cmd_notes(args):
    with open(args.report) as f:
        report = json.load(f)
    block = notes_block(report, args.assets.rstrip('/'))
    if args.splice:
        with open(args.splice) as f:
            notes = f.read()
        print(splice_notes(notes, block), end='')
    else:
        print(block)
    return 0


# --- self-test: the whole scan against a fake VirusTotal, no key, no network --------------

class FakeVT(Client):
    """Answers like VirusTotal: one file known and flagged, one known and clean, the rest
    unknown (uploaded, queued once, then completed); one 429 along the way."""

    def __init__(self, known):
        self.now = 0.0
        super().__init__('fake', interval=15.5, sleep=self._sleep, clock=lambda: self.now)
        self.known, self.polls, self.log = known, {}, []
        self.throttled = False

    def _sleep(self, s):
        self.now += s

    def _send(self, req):
        m, url = req.get_method(), req.full_url
        self.log.append((m, url, self.now))
        if not self.throttled and m == 'POST':
            self.throttled = True
            return 429, b'{"error": {"code": "QuotaExceededError"}}'
        path = url[len(API):] if url.startswith(API) else url
        if m == 'GET' and path.startswith('/files/') and path != '/files/upload_url':
            sha = path.split('/')[2]
            if sha not in self.known:
                return 404, b'{"error": {"code": "NotFoundError"}}'
            return 200, json.dumps({'data': {'attributes': self.known[sha]}}).encode()
        if m == 'GET' and path == '/files/upload_url':
            return 200, b'{"data": "https://upload.example/big"}'
        if m == 'POST' and (path == '/files' or url == 'https://upload.example/big'):
            assert b'name="file"' in req.data, 'multipart body'
            aid = f'a{len(self.polls)}'
            self.polls[aid] = 0
            return 200, json.dumps({'data': {'type': 'analysis', 'id': aid}}).encode()
        if m == 'GET' and path.startswith('/analyses/'):
            aid = path.split('/')[2]
            self.polls[aid] += 1
            if self.polls[aid] < 2:
                return 200, b'{"data": {"attributes": {"status": "queued"}}}'
            return 200, json.dumps({'data': {'attributes': {
                'status': 'completed', 'date': 1700000000,
                'stats': {'malicious': 0, 'suspicious': 1, 'undetected': 60,
                          'harmless': 0, 'type-unsupported': 10},
                'results': {'Ikarus': {'category': 'suspicious', 'result': 'Trojan.Generic'},
                            'ESET-NOD32': {'category': 'undetected', 'result': None}}}}}
                ).encode()
        return 400, b'{"error": "unexpected"}'


def selftest():
    import contextlib
    tmp = tempfile.mkdtemp()
    zpath = os.path.join(tmp, 'armada2-refit-9.9.9.zip')
    mz = lambda tag: b'MZ\x90\x00' + tag.encode() + b'\x00' * 64
    with zipfile.ZipFile(zpath, 'w') as z:
        z.writestr('armada2-refit-9.9.9/manifest.json',
                   json.dumps({'name': 'armada2-refit', 'version': '9.9.9', 'commit': 'abc1234'}))
        z.writestr('armada2-refit-9.9.9/game/', '')
        z.writestr('armada2-refit-9.9.9/game/Menus.asi', mz('menus'))
        z.writestr('armada2-refit-9.9.9/game/Menus.ini', 'not a PE')
        z.writestr('armada2-refit-9.9.9/game/binkw32.dll', mz('bink'))
        z.writestr('armada2-refit-9.9.9/game/fake.dll', b'not MZ at all')
        z.writestr('armada2-refit-9.9.9/chain/d3d8.dll', mz('chain'))
        z.writestr('armada2-refit-9.9.9/vendor/Patch/winmm.dll', mz('winmm'))
        z.writestr('armada2-refit-9.9.9/vendor/Patch/LICENSE', 'MIT')
    exe = os.path.join(tmp, 'armada2-refit-installer-windows.exe')
    with open(exe, 'wb') as f:
        f.write(mz('installer') + b'\x00' * (BIG + 1))           # takes the upload URL
    sha = lambda b: hashlib.sha256(b).hexdigest()
    known = {
        sha(mz('menus')): {'last_analysis_date': 1700000000,
                           'last_analysis_stats': {'malicious': 2, 'suspicious': 0,
                                                   'undetected': 68, 'harmless': 0},
                           'last_analysis_results': {
                               'BitDefender': {'category': 'malicious', 'result': 'Gen:Variant.Zusy.1'},
                               'MicroWorld-eScan': {'category': 'malicious', 'result': 'Gen:Variant.Zusy.1'},
                               'Microsoft': {'category': 'undetected', 'result': None}}},
        sha(mz('winmm')): {'last_analysis_date': 1700000000,
                           'last_analysis_stats': {'malicious': 0, 'undetected': 70},
                           'last_analysis_results': {'Microsoft': {'category': 'undetected'}}},
    }
    fake = FakeVT(known)
    out = os.path.join(tmp, 'out')
    summary = os.path.join(tmp, 'summary.md')
    os.environ['GITHUB_STEP_SUMMARY'] = summary
    os.environ.pop('VT_FAIL_ABOVE', None)
    args = argparse.Namespace(zip=zpath, extra=[exe], out=out, commit=None,
                              run_url='https://ci.example/run/1', timeout=1800)
    with contextlib.redirect_stdout(io.StringIO()):
        rc = cmd_scan(args, client=fake)
    assert rc == 0, rc
    report = json.load(open(os.path.join(out, 'virustotal.json')))
    paths = [e['path'] for e in report['files']]
    assert paths == ['armada2-refit-installer-windows.exe', 'chain/d3d8.dll',
                     'game/binkw32.dll', 'game/Menus.asi', 'vendor/Patch/winmm.dll'], paths
    assert report['version'] == '9.9.9' and report['commit'] == 'abc1234', report
    by = {e['path']: e for e in report['files']}
    m = by['game/Menus.asi']
    assert (m['source'], m['malicious'], m['total']) == ('known', 2, 70), m
    assert [d['engine'] for d in m['detections']] == ['BitDefender', 'MicroWorld-eScan'], m
    b = by['game/binkw32.dll']
    assert (b['source'], b['status'], b['malicious'], b['suspicious'], b['total']) == \
        ('uploaded', 'done', 0, 1, 61), b
    assert b['detections'] == [{'engine': 'Ikarus', 'category': 'suspicious',
                                'result': 'Trojan.Generic'}], b
    assert all(e['status'] == 'done' for e in report['files']), report
    calls = [(mth, u) for mth, u, _ in fake.log]
    assert ('GET', API + '/files/upload_url') in calls, 'big file takes an upload URL'
    assert ('POST', 'https://upload.example/big') in calls
    times = [t for _, _, t in fake.log]
    assert all(b - a >= 15.5 - 1e-9 for a, b in zip(times, times[1:])), 'rate limit'
    assert any(t2 - t1 >= 60 for t1, t2 in zip(times, times[1:])), '429 backs off'
    md = open(os.path.join(out, 'virustotal.md')).read()
    assert '| `game/Menus.asi` |' in md and 'BitDefender: Gen:Variant.Zusy.1' in md, md
    assert '0/61 (+1 suspicious)' in md and 'Ikarus: Trojan.Generic (suspicious)' in md, md
    assert '4 of 5 files flagged' in md, md
    assert open(summary).read().startswith('## VirusTotal'), 'step summary'
    # The gate: off when unset or empty, on above the threshold.
    os.environ['VT_FAIL_ABOVE'] = ''
    assert gate(report) == 0
    os.environ['VT_FAIL_ABOVE'] = '2'
    assert gate(report) == 0
    os.environ['VT_FAIL_ABOVE'] = '1'
    with contextlib.redirect_stdout(io.StringIO()) as o:
        assert gate(report) == 3
    assert '::error::game/Menus.asi' in o.getvalue()
    os.environ.pop('VT_FAIL_ABOVE')
    # A timeout leaves files pending, not failed.
    fake2 = FakeVT({})
    fake2.throttled = True
    entries = scan(fake2, [('game/X.asi', 'X.asi', mz('x'))], timeout=1,
                   clock=lambda: fake2.now)
    assert entries[0]['status'] == 'pending' and ratio(entries[0]) == 'pending', entries
    # No key: a notice, exit 0, no reports.
    os.environ.pop('VT_API_KEY', None)
    out2 = os.path.join(tmp, 'nokey')
    with contextlib.redirect_stdout(io.StringIO()) as o:
        assert cmd_scan(argparse.Namespace(zip=zpath, extra=[], out=out2, commit=None,
                                           run_url=None, timeout=1)) == 0
    assert o.getvalue().startswith('::notice::') and not os.path.exists(out2)
    # Release notes: appended once, replaced on a re-scan.
    block = notes_block(report, 'https://example/releases/download/v9.9.9')
    assert '[Menus.asi](https://www.virustotal.com/gui/file/' in block and ' 2/70' in block
    n1 = splice_notes('Notes.\n', block)
    n2 = splice_notes(n1, block.replace('2/70', '3/70'))
    assert n2.count(START) == 1 and '3/70' in n2 and n2.startswith('Notes.\n\n'), n2
    log(f'selftest: ok ({len(fake.log)} fake requests)')
    return 0


def main():
    if sys.argv[1:] == ['--selftest']:
        return selftest()
    p = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = p.add_subparsers(dest='cmd', required=True)
    s = sub.add_parser('scan')
    s.add_argument('--zip', required=True)
    s.add_argument('--extra', action='append', help='another file to scan (repeatable)')
    s.add_argument('--out', default='.')
    s.add_argument('--commit', help="default: the zip manifest's")
    s.add_argument('--run-url')
    s.add_argument('--timeout', type=float, default=1800,
                   help='seconds to wait for uploaded files to be analysed')
    n = sub.add_parser('notes')
    n.add_argument('report')
    n.add_argument('--assets', required=True, help='URL the reports are attached under')
    n.add_argument('--splice', help='release notes to put the block into (printed)')
    a = p.parse_args()
    return cmd_scan(a) if a.cmd == 'scan' else cmd_notes(a)


if __name__ == '__main__':
    sys.exit(main())
