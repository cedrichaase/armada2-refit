"""The step log: every action, check and note a session takes, as JSON lines (for the
report) and as plain text (for a human with `less`).  Appending only, so several
processes -- the runner, an agent driving `a2test drive`, a person at a terminal -- can
write to one session without coordinating."""
import datetime
import json
from pathlib import Path


class Log:
    def __init__(self, d):
        self.dir = Path(d)
        self.jsonl = self.dir / 'steps.jsonl'
        self.txt = self.dir / 'log.txt'

    def _write(self, rec):
        rec.setdefault('t', datetime.datetime.now().isoformat(timespec='milliseconds'))
        with open(self.jsonl, 'a') as f:
            f.write(json.dumps(rec) + '\n')
        mark = {'pass': 'PASS', 'fail': 'FAIL', 'warn': 'WARN', 'skip': 'SKIP',
                'review': 'REVIEW', 'inconclusive': 'INCONCLUSIVE'}.get(rec.get('status'), '')
        line = f"{rec['t'][11:23]}  {rec['kind']:<7} {mark:<5} {rec.get('text', '')}"
        extra = []
        if rec.get('detail'):
            extra += ['    ' + l for l in str(rec['detail']).splitlines()]
        for s in rec.get('shots', []):
            extra.append(f'    shot: {s}')
        with open(self.txt, 'a') as f:
            f.write(line.rstrip() + '\n' + ''.join(e + '\n' for e in extra))

    def meta(self, **kw):
        self._write(dict(kind='meta', text=', '.join(f'{k}={v}' for k, v in kw.items()), meta=kw))

    def step(self, n, text):
        self._write(dict(kind='step', n=n, text=text))

    def action(self, text, status='ok', detail=None, shots=()):
        self._write(dict(kind='action', text=text, status=status, detail=detail,
                         shots=[_rel(self.dir, s) for s in shots]))

    def check(self, text, status, detail=None, shots=(), how=None, measure=None):
        self._write(dict(kind='check', text=text, status=status, detail=detail, how=how,
                         measure=measure, shots=[_rel(self.dir, s) for s in shots]))

    def shot(self, path, name):
        self._write(dict(kind='shot', text=name, shots=[_rel(self.dir, path)]))

    def note(self, text, status=None, detail=None):
        self._write(dict(kind='note', text=text, status=status, detail=detail))

    def records(self):
        if not self.jsonl.exists():
            return []
        return [json.loads(l) for l in self.jsonl.read_text().splitlines() if l.strip()]


def _rel(base, p):
    try:
        return str(Path(p).resolve().relative_to(Path(base).resolve()))
    except ValueError:
        return str(p)
