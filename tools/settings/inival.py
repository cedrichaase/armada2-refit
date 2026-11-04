"""What the settings tools read of an ini and of an overlay's names: the value a plugin
would read (Ini::string) and the names the overlay's edit (crates/a2ini) accepts. Reading
only; every edit goes through the a2ini host tool (host.sh)."""

_WS = ' \t\r\n\x0b\x0c'


def read(path):
    try:
        with open(path, 'rb') as f:
            return f.read().decode('latin-1')
    except FileNotFoundError:
        return ''


def valid_value(v):
    return '\r' not in v and '\n' not in v and v == v.strip(_WS)


def valid_section(s):
    return bool(s) and s == s.strip(_WS) and not any(c in s for c in '\r\n]')


def valid_key(k):
    return bool(k) and k == k.strip(_WS) and not any(c in k for c in '\r\n=') and k[0] not in ';#['


def value(text, section, key):
    """The key's value as a plugin reads it: the first in the section (names without
    ASCII case, a section written twice is one), a trailing ; comment and whitespace
    dropped; None when absent."""
    cur = None
    sec, k = section.lower(), key.lower()
    for line in text.replace('\r\n', '\n').split('\n'):
        t = line.strip(_WS)
        if t.startswith('['):
            cur = t[1:].split(']')[0].strip(_WS).lower()
        elif cur == sec and t and t[0] not in ';#' and '=' in t:
            name, v = t.split('=', 1)
            if name.strip(_WS).lower() == k:
                return v.split(';', 1)[0].strip(_WS)
    return None
