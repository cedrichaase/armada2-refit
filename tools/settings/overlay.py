#!/usr/bin/env python3
"""RefitPlayer.ini, the player's settings overlay: the same edit as `a2ini::edit`
(crates/a2ini), for the bench and the GUI installer. Both pass the fixtures in
tools/settings/fixtures/, which are the spec (tools/settings/README.md, "The writer").

    overlay.py FILE WORD... [--unset S:K ...]   set (and remove) keys of FILE
    overlay.py --get FILE S:K                   print a key's value (exit 1 if absent)
    overlay.py --fixtures DIR                   check edit() against every fixture

A WORD is `Section:Key=value`, the section spelled as in the overlay
(`Render3D.Modules:sky=0`, `UI.HUD:BarHidden=1`). The file is written whole to
`FILE.tmp` and renamed over FILE, as UI.asi writes it; when nothing changes it is
not written at all.

The text is handled as latin-1, so every byte that is not touched is kept as it was.
"""
import json
import os
import sys

HEADER = (
    '; RefitPlayer.ini: your choices from the Refit screen, written by UI.asi. Installers never touch it.',
    '; Delete a line to go back to the shipped setting.',
)

_WS = ' \t\r\n\x0b\x0c'
_LOWER = str.maketrans('ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')


def _trim(s):
    return s.strip(_WS)


def _eq(a, b):
    """Names compare without regard to ASCII case (GetPrivateProfileString's rule)."""
    return a.translate(_LOWER) == b.translate(_LOWER)


def valid_value(v):
    """Rule 9: no CR or LF, no leading or trailing whitespace."""
    return '\r' not in v and '\n' not in v and v == _trim(v)


def valid_section(s):
    return bool(s) and s == _trim(s) and not any(c in s for c in '\r\n]')


def valid_key(k):
    return bool(k) and k == _trim(k) and not any(c in k for c in '\r\n=') and k[0] not in ';#['


def _split(text):
    """[content, terminator] per line; the terminator is '\\r\\n', '\\n' or ''."""
    out = []
    i = 0
    while i < len(text):
        j = text.find('\n', i)
        if j < 0:
            out.append([text[i:], ''])
            break
        if j > i and text[j - 1] == '\r':
            out.append([text[i:j - 1], '\r\n'])
        else:
            out.append([text[i:j], '\n'])
        i = j + 1
    return out


def _eol(text):
    """Rule 1: the file's first line ending; CRLF for a file that has none."""
    j = text.find('\n')
    if j < 0:
        return '\r\n'
    return '\r\n' if j > 0 and text[j - 1] == '\r' else '\n'


def _header(content):
    t = _trim(content)
    if not t.startswith('['):
        return None
    return _trim(t[1:].split(']', 1)[0])


def _key(content):
    """The key of a key line (rule 2), else None."""
    t = _trim(content)
    if not t or t[0] in ';#[' or '=' not in t:
        return None
    return _trim(t.split('=', 1)[0])


def _blank(content):
    return _trim(content) == ''


def _occurrences(lines, section):
    """(header index, end index) of every occurrence of the section."""
    occ = []
    for i, (c, _) in enumerate(lines):
        h = _header(c)
        if h is None:
            continue
        if occ and occ[-1][1] is None:
            occ[-1][1] = i
        if _eq(h, section):
            occ.append([i, None])
    if occ and occ[-1][1] is None:
        occ[-1][1] = len(lines)
    return occ


def edit(text, section, key, value):
    """The text of the overlay with [section] key set to value, or removed when value
    is None; the input unchanged when nothing changes or the value is refused."""
    if not valid_section(section) or not valid_key(key):
        return text
    if value is not None and not valid_value(value):
        return text
    eol = _eol(text)
    lines = _split(text)
    occ = _occurrences(lines, section)
    matches = [i for h, e in occ for i in range(h + 1, e)
               if (k := _key(lines[i][0])) is not None and _eq(k, key)]
    if value is not None:
        if matches:
            first = matches[0]
            c = lines[first][0]
            lines[first][0] = c[:c.index('=') + 1] + value
            for i in reversed(matches[1:]):
                del lines[i]
        elif occ:
            h, e = occ[-1]
            at = h
            for i in range(h + 1, e):
                if _key(lines[i][0]) is not None:
                    at = i
            if not lines[at][1]:
                lines[at][1] = eol
            lines.insert(at + 1, [f'{key}={value}', eol])
        else:
            if not text:
                lines = [[h, eol] for h in HEADER]
            if lines and not _blank(lines[-1][0]):
                if not lines[-1][1]:
                    lines[-1][1] = eol
                lines.append(['', eol])
            elif lines and not lines[-1][1]:
                lines[-1][1] = eol
            lines.append([f'[{section}]', eol])
            lines.append([f'{key}={value}', eol])
    else:
        drop = set(matches)
        for h, e in occ:
            if not any(h < i < e for i in drop):
                continue
            rest = [i for i in range(h + 1, e) if i not in drop]
            if all(_blank(lines[i][0]) for i in rest):
                drop.add(h)
                drop.update(rest)
        lines = [l for i, l in enumerate(lines) if i not in drop]
    out = ''.join(c + t for c, t in lines)
    if out == text:
        return text
    if lines and not lines[-1][1]:
        out += eol
    return out


def read(path):
    try:
        with open(path, 'rb') as f:
            return f.read().decode('latin-1')
    except FileNotFoundError:
        return ''


def apply(path, section, key, value):
    """Edits the file at path; True when it was written."""
    old = read(path)
    new = edit(old, section, key, value)
    if new == old:
        return False
    tmp = path + '.tmp'
    with open(tmp, 'wb') as f:
        f.write(new.encode('latin-1'))
    os.replace(tmp, path)
    return True


def get(text, section, key):
    """A key's value as GetPrivateProfileString reads it: the first in the section,
    whitespace and one pair of quotes dropped; None when absent."""
    cur = None
    for c, _ in _split(text):
        h = _header(c)
        if h is not None:
            cur = h
            continue
        t = _trim(c)
        if not t or t.startswith(';') or '=' not in t or cur is None or not _eq(cur, section):
            continue
        k, v = t.split('=', 1)
        if _eq(_trim(k), key):
            v = _trim(v)
            if len(v) >= 2 and v[0] == v[-1] and v[0] in '"\'':
                v = v[1:-1]
            return v
    return None


def parse_word(word, with_value=True):
    """`Section:Key=value` -> (section, key, value); `Section:Key` with with_value False."""
    sec, sep, rest = word.partition(':')
    if not sep or not sec:
        raise ValueError(f'{word}: want Section:Key=value')
    if not with_value:
        return sec, rest, None
    key, sep, val = rest.partition('=')
    if not sep or not key:
        raise ValueError(f'{word}: want Section:Key=value')
    return sec, key, val


def fixture_args(path):
    """A fixture's .args: a JSON array [section, key, value or null]."""
    with open(path, encoding='utf-8') as f:
        sec, key, val = json.load(f)
    return sec, key, val


def run_fixtures(d):
    names = sorted(n[:-3] for n in os.listdir(d) if n.endswith('.in'))
    bad = 0
    for n in names:
        base = os.path.join(d, n)
        sec, key, val = fixture_args(base + '.args')
        with open(base + '.in', 'rb') as f:
            src = f.read().decode('latin-1')
        with open(base + '.out', 'rb') as f:
            want = f.read().decode('latin-1')
        got = edit(src, sec, key, val)
        if got != want:
            bad += 1
            print(f'FAIL {n}\n  want {want!r}\n  got  {got!r}')
    print(f'overlay.py: {len(names) - bad} of {len(names)} fixtures pass')
    return 1 if bad or not names else 0


def main(argv):
    if len(argv) == 2 and argv[0] == '--fixtures':
        return run_fixtures(argv[1])
    if len(argv) == 3 and argv[0] == '--get':
        sec, key, _ = parse_word(argv[2], with_value=False)
        v = get(read(argv[1]), sec, key)
        if v is None:
            return 1
        print(v)
        return 0
    if not argv or argv[0].startswith('-'):
        print(__doc__.strip(), file=sys.stderr)
        return 2
    path, words = argv[0], argv[1:]
    todo = []
    i = 0
    while i < len(words):
        if words[i] == '--unset':
            todo.append(parse_word(words[i + 1], with_value=False))
            i += 2
        else:
            todo.append(parse_word(words[i]))
            i += 1
    for sec, key, val in todo:
        if val is not None and not valid_value(val):
            print(f'{sec}:{key}: value {val!r} refused (CR, LF or surrounding whitespace)', file=sys.stderr)
            return 1
        wrote = apply(path, sec, key, val)
        what = f'{key}={val}' if val is not None else f'{key} removed'
        print(f'{os.path.basename(path)}: [{sec}] {what}{"" if wrote else " (unchanged)"}')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
