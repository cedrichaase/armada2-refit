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
- Desktop shots (`Kind: desktop`, `Run:`, `Wallpaper:`, the `run` command): a program,
  the GUI installer, filmed on a bare bench desktop, installing into a stock clone of the
  game. The film field `Pointer:` (1920x1080): pointer coordinates scaled to each take.
- Two games in one shot (`Players:`, `Prepare:`, `PLAYER: CMD` lines, `typelog`), filmed
  together and laid side by side by `split [w]`: online play, host and joiner.
- `./a2film check`, `status`, `probe` (1280x720, `--jobs 4`), `take` (3840x2160,
  `--jobs 1`), `sheet`, `animatic` and `manifest` (`a2film-manifest/1`). A take is
  re-filmed only when its spec changes.
- `films/selftest` and the draft boards `films/whats-new` and `films/promo` (with its review plan, `PLAN.md`).
- The `film` skill (`.claude/skills/film/SKILL.md`).

Tooling only; nothing to see in game.
