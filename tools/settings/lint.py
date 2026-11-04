#!/usr/bin/env python3
"""Checks the settings tables against the schema, each other, the shipped inis and the
sources (tools/settings/README.md, "The lint").

    lint.py                  every settings table of the repository
    lint.py TABLE.toml ...   those tables only (the selftest's fixtures)

1. The schema: kind, apply, scope, tab and flags are known; a BOOL's default is 0 or 1;
   an ENUM's default is one of its `values`; `labels` has as many entries as `values`;
   label at most 40 characters, help at most 200, both printable ASCII.
2. No (file, section, key) appears twice across the tables.
3. Each row's key is in the shipped ini, in its section, with the value `default`
   (compared trimmed, without regard to case), unless the row is NOT_SHIPPED.
4. A NOT_SHIPPED key is absent from the shipped ini.
5. The table's generated file is up to date (gen.py).
6. A `moved_from` is `Stem.Section.Key` and not the row's own key.
7. Within one plugin's table and its fragments (`module = "<m>"`, checked as one table:
   2 across them, 3 against the plugin's one shipped ini), a section has one owning
   table, except those the plugin table lists in `shared_sections`, whose keys are split.
Exits 1 with one line per problem.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen  # noqa: E402
import inival  # noqa: E402

shipped_value = inival.value


def lint(paths):
    problems = []
    seen = {}
    owners = {}  # (plugin, file, section) -> (table, shared)
    for path in paths:
        r = gen.rel(path)
        try:
            t = gen.load(path)
            rows = gen.rows_of(t, path)
        except gen.TableError as e:
            problems.append(str(e))
            continue
        except Exception as e:  # tomllib's errors
            problems.append(f'{r}: {e}')
            continue
        try:
            base = gen.base_of(path) if 'module' in t else path
            shipped_rel = gen.shipped_of(path)
            shared = {x.lower() for x in gen.load(base)['shared_sections']}
        except gen.TableError as e:
            problems.append(str(e))
            continue
        shipped = os.path.join(gen.ROOT, shipped_rel)
        if not os.path.isfile(shipped):
            problems.append(f'{r}: shipped ini {shipped_rel} not found')
            text = None
        else:
            text = inival.read(shipped)
            if os.path.basename(shipped).lower() != t['file'].lower():
                problems.append(f'{r}: file {t["file"]} is not the shipped ini\'s name ({os.path.basename(shipped)})')
        raw = {row.get('name'): row for row in t['row']}
        gen_out = gen.output_path(path)
        for name, row in rows:
            where = f'{r} row `{name}`'
            src = raw[name]
            # 1. the schema (codes are checked by gen.rows_of)
            values = row['values'].split('|') if row['values'] is not None else None
            labels = row['value_labels'].split('|') if row['value_labels'] is not None else None
            if row['kind'] == 'BOOL':
                if row['def'] not in ('0', '1'):
                    problems.append(f'{where}: a BOOL\'s default is 0 or 1, not `{row["def"]}`')
                if values is not None and values != ['0', '1']:
                    problems.append(f'{where}: a BOOL has no values of its own')
                if labels is not None and len(labels) != 2:
                    problems.append(f'{where}: a BOOL\'s labels are two')
            else:
                if not values or any(v == '' for v in values):
                    problems.append(f'{where}: an ENUM needs `values`')
                elif row['def'] not in values:
                    problems.append(f'{where}: default `{row["def"]}` is not one of its values {values}')
                if values and len(set(values)) != len(values):
                    problems.append(f'{where}: a value is listed twice')
                if values and (labels is None or len(labels) != len(values)):
                    problems.append(f'{where}: `labels` must have {len(values)} entries, as `values` has')
            for k, mx in (('label', gen.LABEL_MAX), ('help', gen.HELP_MAX)):
                v = row[k]
                if len(v) > mx:
                    problems.append(f'{where}: {k} is {len(v)} characters, at most {mx}')
                if any(ord(c) < 32 or ord(c) > 126 for c in v):
                    problems.append(f'{where}: {k} must be printable ASCII')
            if not row['label']:
                problems.append(f'{where}: label is empty')
            if 'help' not in src:
                problems.append(f'{where}: help missing (write "" for none)')
            for k in ('section', 'key', 'def'):
                if not inival.valid_value(row[k]) or (k == 'section' and not inival.valid_section(row[k])) \
                        or (k == 'key' and not inival.valid_key(row[k])):
                    problems.append(f'{where}: {k} `{row[k]}` cannot be written to an ini')
            # 2. one row per (file, section, key)
            ident = (row['file'].lower(), row['section'].lower(), row['key'].lower())
            if ident in seen:
                problems.append(f'{where}: [{row["section"]}] {row["key"]} of {row["file"]} is also {seen[ident]}')
            else:
                seen[ident] = where
            # 7. one owning table per section
            own = (t['plugin'], row['file'].lower(), row['section'].lower())
            if own in owners and owners[own] != r and row['section'].lower() not in shared:
                problems.append(f'{where}: section [{row["section"]}] is owned by {owners[own]}; a section has one owning table (or is in `shared_sections`)')
            owners.setdefault(own, r)
            # 3 and 4. the shipped ini
            if text is not None:
                got = shipped_value(text, row['section'], row['key'])
                if 'NOT_SHIPPED' in row['flags']:
                    if got is not None:
                        problems.append(f'{where}: NOT_SHIPPED, but {shipped_rel} has [{row["section"]}] {row["key"]}={got}')
                elif got is None:
                    problems.append(f'{where}: {shipped_rel} has no [{row["section"]}] {row["key"]}')
                elif got.lower() != row['def'].lower():
                    problems.append(f'{where}: {shipped_rel} has {row["key"]}={got}, the table says {row["def"]}')
            # 6. a moved key is a key of another place
            mf = row['moved_from']
            if mf is not None:
                stem = row['file'][:-4] if row['file'].lower().endswith('.ini') else row['file']
                if mf.lower() == f'{stem}.{row["section"]}.{row["key"]}'.lower():
                    problems.append(f'{where}: moved_from `{mf}` is the row\'s own key')
        # 5. the generated file
        try:
            want = gen.emit(path)
        except gen.TableError as e:
            problems.append(str(e))
        else:
            if gen.current(gen_out) != want:
                problems.append(f'{r}: {gen.rel(gen_out)} is out of date (run tools/settings/gen.py)')
    return problems


def main(argv):
    paths = [os.path.abspath(p) for p in argv] if argv else gen.tables()
    problems = lint(paths)
    for p in problems:
        print(f'lint.py: {p}')
    if not problems:
        print(f'lint.py: {len(paths)} tables pass')
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
