# Version resources

Every PE file we build carries a `VERSIONINFO` resource: the eight ASI plugins and the
cutscene proxy `binkw32.dll`. The files `a2mod` never switches (DXVK, d3d8to9, the ASI
loader, the widescreen patch) are vendored and stay exactly as their authors built
them.

## Why

GitHub issue #4: Bitdefender quarantined `Menus.asi`, `QOL.asi`, `QOLRules.asi` and
`Online.asi` out of the release cache, and the installer failed without saying why. An
unsigned DLL that hooks another process and names no company, product or description is
what antivirus heuristics look for. A version resource alone does not clear a detection
(code signing would do more), but it is cheap. It also tells a player in Explorer's
Properties what the file is and which release it came from. The plan is
`plans/issue-4-installer-antivirus.md`, part 3.

## What it holds

| Field | Value |
|---|---|
| `CompanyName` | Armada II Refit project |
| `ProductName` | Armada II Refit |
| `FileDescription` | "Armada II Refit: …", given by the build script |
| `FileVersion` | the layer's version, from its `CHANGELOG.md` |
| `ProductVersion` | the project's version, from the root `CHANGELOG.md` |
| `OriginalFilename`, `InternalName` | the file's name, and without its extension |
| `LegalCopyright` | `LICENSE`'s copyright line, then "MIT License" |
| `Comments` | the repository URL |

## How

- `version.sh` is sourced for `version FILE`, the X.Y.Z of a changelog's newest entry.
  `publish/package.sh` uses it too.
- `build-res.sh <out.res> <OriginalFilename> <FileDescription> <layer version>
  <project version>` fills `versioninfo.rc.in` and compiles it with `llvm-rc
  -no-preprocess`. Without the preprocessor there is no `winver.h`, so the template spells
  its constants as numbers. No clang is needed, which the `render3d` CI job doesn't have.
- The C plugins' `build.sh` run it and hand the `.res` to `lld-link` beside the object,
  which links it as the `.rsrc` section.
- `render3d/build.rs` and `ui/build.rs` run it into `OUT_DIR` and pass the `.res` as a
  `rustc-link-arg`. They re-run when the template, `LICENSE` or either changelog changes,
  so a version bump reaches the DLL without a clean build.

A plugin folded into another (`ui/PLAN.md`) needs only one call fewer.

## Checks

- `publish/package.sh` refuses to package any `game/*.asi` or `game/*.dll` whose
  `llvm-readobj --coff-resources` shows no `VERSIONINFO`.
- `publish/installer/test.ps1` reads each one with `(Get-Item f).VersionInfo`, as
  Windows does: the description, the product name and file name, and a product version
  equal to the package's.

## Not done yet

`lld-link` also honours `/brepro` (a content hash where the link time would be) and
`/release` (the PE checksum, otherwise 0). Neither is on. Whether either changes a
detection has to be measured by scanning one ASI with and without on VirusTotal, which
waits for the scan stage (part 2a of the plan).
