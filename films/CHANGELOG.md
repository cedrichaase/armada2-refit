# Changelog — films

`./a2film`: storyboards filmed on the test bench and cut into a rough cut and an edit
manifest. It installs nothing into the game: every take runs on a bench clone. Versioning
rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The why is
in [`README.md`](README.md).

## 1.0.0 — 2026-10-14

### Added
- The storyboard format (`films/<name>/storyboard.md`): sections and shots with scene,
  variants, camera (```` ```setup ````, ```` ```action ````), layout, crop, transition,
  titles, key hints, audio, music beds per section, cards, `From:` footage and a status.
- `./a2film check`, `status`, `probe` (1280x720, `--jobs 4`), `take` (3840x2160,
  `--jobs 1`), `sheet`, `animatic` and `manifest` (`a2film-manifest/1`). A take is
  re-filmed only when its spec changes.
- `films/selftest` and the first draft board, `films/whats-new`.
- The `film` skill (`.claude/skills/film/SKILL.md`).

Tooling only; nothing to see in game.
