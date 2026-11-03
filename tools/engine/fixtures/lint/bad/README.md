# lint fixtures: one violation each

Each directory is the good set with one change; `expect.txt` holds a phrase the lint
must print for it. `lint.py --selftest` runs them all. A fixture's `readme = "./x"` is relative to its
profile set directory; its sources are in `src/` and run with `--src-root`.
