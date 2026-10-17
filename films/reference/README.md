# Reference — scouted moments for the cut

A reference is a moment found while scouting, worth cutting into a film as it is: a few
seconds of a mission's intro cinematic, a fleet in a stock mission, a take of a scratch
board. The catalogue says where each one is and what it is like, so a board can use it
by id and an editor can judge it without watching the footage first. Like the boards,
the catalogue is a recipe and is committed; the footage it points to is an asset and
lives in `$A2_DATA`, never in a checkout.

Every `.md` file in this folder other than this README is a catalogue file, so several
scouting runs can each write their own (`intros.md`, `missions.md`). Ids are unique
across all of them; `./a2film check` says where an id is used twice.

## An entry

An entry is a `###` heading, `<id> · <name>`, then its fields, then its description in
prose, in the storyboard's format (`films/README.md`, "The storyboard"). Other headings
and prose are notes for the reader.

    ### borg01-intro · The cube over the colony
    Take: intros-scout m01 refit probe
    Mission: a2_borg01, the intro cinematic, after the title
    In: 12.5
    Out: 20.0
    Audio: the cinematic's score throughout; Picard's line at 15 s
    Quality: good
    Shows: models, lighting

    The cube fills the frame from the left and turns towards the camera ...

| Key | |
|---|---|
| `Take` | `<film> <shot> [variant] [tier]`: a take in `$A2_DATA/films/<film>/`, refit and probe unless named. The film need not have a board on this branch (a scouting board) |
| `File` | a file instead, a path relative to `$A2_DATA`. An entry names a `Take` or a `File` |
| `Mission` | where the moment is in the game and how to get there again: the mission, the cinematic, the moment. For the reader, and for filming it again at another tier |
| `In`, `Out` | seconds into the file where the moment starts and ends (`In` 0, `Out` its end by default) |
| `Audio` | what is in its sound: score, dialogue and when, effects; whether it is usable under a bed |
| `Quality` | `good`, `usable`, `poor`, then a note (`usable: 720p, a frame drop at 14 s`); `example` marks an example, which `check` reports when a shot uses it |
| `Shows` | the features it shows, as a shot's `Shows` |

## Using one

A footage shot names it:

    ### i01 · The cube over the colony
    Kind: footage
    From: ref:borg01-intro
    In: 1.5
    Length: 4
    Audio: none

The shot's `In:` counts from the entry's `In`, so `In: 1.5` above starts 14 s into the
take. Without a `Length:` it runs to the entry's `Out`. The animatic, the contact sheet
and the manifest resolve the entry to its file; the manifest also carries its `ref`,
`out`, `quality` and audio notes. A reference is not filmed, so it is never stale: after
re-scouting a moment, update its entry.
