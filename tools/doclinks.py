#!/usr/bin/env python3
"""Check the relative links and anchors of every tracked *.md.

    tools/doclinks.py            check every tracked .md (a walk outside a checkout)
    tools/doclinks.py FILE...    check only these

A link is `[text](target)` or `[text]: target`. http(s), mailto and the like are not
followed. A relative target must exist (file or directory), and a `#anchor` on a
Markdown target must match a heading's GitHub slug or an `id="..."` in that file.
Code fences and inline code are skipped. Exit 1 if anything is broken.
"""
import os
import re
import subprocess
import sys
from urllib.parse import unquote

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

LINK = re.compile(
    r'(?<!\!)\[(?:[^\[\]]|\[[^\]]*\])*\]\(\s*<?([^)\s>]+)>?(?:\s+"[^"]*")?\s*\)'
    r'|^\s*\[[^\]]+\]:\s*(\S+)', re.M)
INLINE_CODE = re.compile(r'`+[^`\n]*`+')
HEADING = re.compile(r'^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$')
HTML_ID = re.compile(r'''\b(?:id|name)\s*=\s*["']([^"']+)["']''')
FENCE = re.compile(r'^\s{0,3}(`{3,}|~{3,})')


def md_files():
    try:
        out = subprocess.run(['git', '-C', ROOT, 'ls-files', '*.md'], capture_output=True,
                             text=True, check=True).stdout.split('\n')
        files = [f for f in out if f]
    except Exception:
        files = []
        for d, ds, fs in os.walk(ROOT):
            ds[:] = [x for x in ds if x not in ('.git', 'target', 'node_modules', '.claude')]
            files += [os.path.relpath(os.path.join(d, f), ROOT) for f in fs if f.endswith('.md')]
    return sorted(f for f in files if os.path.exists(os.path.join(ROOT, f)))


def strip_code(text):
    """The text with fenced blocks and inline code blanked (line numbers kept)."""
    out, fence = [], None
    for line in text.split('\n'):
        m = FENCE.match(line)
        if fence:
            if m and m.group(1)[0] == fence[0] and len(m.group(1)) >= len(fence):
                fence = None
            out.append('')
            continue
        if m:
            fence = m.group(1)
            out.append('')
            continue
        out.append(INLINE_CODE.sub(lambda mm: ' ' * len(mm.group(0)), line))
    return '\n'.join(out)


def slug(h):
    h = re.sub(r'!?\[([^\]]*)\]\([^)]*\)', r'\1', h)
    h = h.replace('`', '').replace('*', '')
    h = re.sub(r'<[^>]+>', '', h)
    h = h.strip().lower()
    h = re.sub(r'[^\w\- ]', '', h, flags=re.UNICODE)
    return h.replace(' ', '-')


_anchors = {}


def anchors(path):
    if path in _anchors:
        return _anchors[path]
    seen, out, fence = {}, set(), None
    try:
        text = open(path, encoding='utf-8', errors='replace').read()
    except OSError:
        text = ''
    for line in text.split('\n'):
        m = FENCE.match(line)
        if fence:
            if m and m.group(1)[0] == fence[0]:
                fence = None
            continue
        if m:
            fence = m.group(1)
            continue
        h = HEADING.match(line)
        if h:
            s = slug(h.group(2))
            n = seen.get(s, 0)
            seen[s] = n + 1
            out.add(s if n == 0 else '%s-%d' % (s, n))
    out.update(HTML_ID.findall(text))
    _anchors[path] = out
    return out


def check(rel):
    path = os.path.join(ROOT, rel)
    text = strip_code(open(path, encoding='utf-8', errors='replace').read())
    bad = []
    for m in LINK.finditer(text):
        target = m.group(1) or m.group(2)
        line = text.count('\n', 0, m.start()) + 1
        if re.match(r'^[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith('//'):
            continue
        file_part, _, frag = target.partition('#')
        file_part = unquote(file_part.split('?')[0])
        if file_part == '':
            dest = path
        elif file_part.startswith('/'):
            dest = os.path.join(ROOT, file_part.lstrip('/'))
        else:
            dest = os.path.normpath(os.path.join(os.path.dirname(path), file_part))
        if not os.path.exists(dest):
            bad.append((line, target, 'no such file'))
            continue
        if frag and os.path.isfile(dest) and dest.endswith('.md'):
            if unquote(frag).lower() not in {a.lower() for a in anchors(dest)}:
                bad.append((line, target, 'no such anchor'))
    return bad


def main(argv):
    files = [os.path.relpath(os.path.abspath(a), ROOT) for a in argv] or md_files()
    n = 0
    for f in files:
        for line, target, why in check(f):
            print('%s:%d: %s: %s' % (f, line, why, target))
            n += 1
    if n:
        print('doclinks: %d broken link(s) in %d file(s)' % (n, len(files)), file=sys.stderr)
        return 1
    print('doclinks: %d file(s), all relative links and anchors resolve' % len(files))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
