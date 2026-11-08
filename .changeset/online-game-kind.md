---
layers: [online]
bump: minor
games: [armada2]
---

### Added
- The online server keeps games of different kinds apart: SV_HOST and SV_JOIN end in the client's game kind (online.kind, a value in the build's profile; 0 for Armada II, and for every client before kinds, whose datagrams lack it). A join code is found only by a joiner of its host's kind; one that sends its kind is told the code is another kind's (SV_ERROR, and the lookup stops), one that does not hears NOTFOUND. Backward compatible both ways: an older server reads past the field.
