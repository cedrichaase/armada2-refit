# lint fixtures: one violation each

A directory here holds only the files its violation changes, plus `expect.txt`, a phrase the
lint must print for it. `lint.py --selftest` copies `../good` to a temporary directory,
overlays the case's files on it (a file named `<path>.delete` removes `<path>`), and lints
the copy. A fixture's `readme = "./x"` is relative to its profile set directory; its sources
are in `src/` and run with `--src-root`.
