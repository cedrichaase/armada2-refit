# Building from source

How to build Armada II Refit's plugins and its release zip yourself, from a fresh
machine to an installed game. Players who just want to play take the zip or the
graphical installer from a [release](https://github.com/cedrichaase/armada2-refit/releases)
([`PLAYING.md`](PLAYING.md)); modders who want the contracts read
[`MODDING.md`](MODDING.md).

The build is a set of bash scripts that cross-compile 32-bit Windows DLLs with Rust and
LLVM. No Visual Studio, MSVC or MinGW is involved. On Windows it runs inside **WSL2 with
Ubuntu 24.04**, the same system CI builds every release on, and the result is the same
zip a release ships, installed with its own `install.bat`. On Linux, skip step 1; on
another distribution, install the equivalents of the packages in step 2.

Building needs nothing from the game. The zip holds code only: no textures, no movies,
nothing derived from the game's files.

## 1. Install WSL2 and Ubuntu 24.04 (Windows)

Use Ubuntu 24.04: it is what CI builds on, and older releases ship an LLVM that lacks
pieces the build uses.

1. Open **PowerShell as Administrator** and run:

       wsl --install -d Ubuntu-24.04

2. Reboot when asked. Ubuntu opens by itself afterwards (otherwise start
   **Ubuntu 24.04** from the Start menu); choose a Linux user name and password.
3. Every command from here on runs **in the Ubuntu terminal**, not in PowerShell or cmd.

If WSL is already installed, `wsl --install -d Ubuntu-24.04` adds that distribution
beside the others. `wsl -l -v` should show it with `VERSION 2`.

## 2. Install the toolchain

You need clang, the LLVM tools (`llvm-dlltool`, `llvm-rc`, `llvm-readobj`), Rust from
rustup with the `i686-pc-windows-msvc` target, and Rust's own `rust-lld` installed as
`lld-link`.

**System packages.** `build-essential` gives Cargo a host C linker for the crates' build
scripts; `git`, `python3` and `zip` are used by the packaging script.

    sudo apt update
    sudo apt install -y build-essential clang lld llvm git python3 zip unzip curl

**Rust, from rustup.** Not Ubuntu's `rustc` or `cargo` packages: those cannot add the
32-bit Windows target.

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
    rustup target add i686-pc-windows-msvc

**`lld-link` must be Rust's `rust-lld`.** Ubuntu's `lld-link` 18 refuses the export
lines rustc writes for a 32-bit DLL (`invalid /export`). Link Rust's copy into
`~/.local/bin` and put that folder first on `PATH`:

    mkdir -p ~/.local/bin
    ln -sf "$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld" ~/.local/bin/lld-link
    echo 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
    source ~/.bashrc

**Check it.** The first line must print your `~/.local/bin/lld-link`, the second must
find every tool, and the third must list `i686-pc-windows-msvc`:

    command -v lld-link
    command -v clang llvm-dlltool llvm-rc llvm-readobj cargo python3 zip
    rustup target list --installed

## 3. Get the source and build

Clone into the Linux home folder, **not** under `/mnt/c/...`: builds on the Windows
drive are many times slower, and a clone made by Git for Windows gets CRLF line endings
that break the bash scripts.

    cd ~
    git clone https://github.com/cedrichaase/armada2-refit.git
    cd armada2-refit

Build every plugin and package them:

    export CARGO="$(command -v cargo)"
    publish/package.sh dist

The first build takes several minutes. The script prints the result's path,
`dist/armada2-refit-<version>.zip`, holding `d3d8.dll` (Display8), `Render3D.asi`,
`UI.asi`, `QOL.asi`, `QOLRules.asi`, `Online.asi`, the cutscene proxy and the
installers (`README.txt` in the zip lists everything).

To rebuild one plugin while working on it, run its own script: `render3d/build.sh`,
`ui/build.sh`, `display/build.sh`, `qol/build.sh`, `rules/build.sh`, `online/build.sh`,
`cutscenes/binkproxy/build.sh`. Each writes into a `build/` folder beside it; the
comment at the top of each script names its output.

To update later: `git pull`, then `publish/package.sh dist` again.

Shaders need no compiler: the compiled SPIR-V and DXIL are committed. dxc
(`tools/shaderc/fetch-dxc.sh`) is needed only to change a shader.

## 4. Install the build into the game

The plugins work with the **GOG release of Star Trek: Armada II, patch 1.1**. On any
other build of `Armada2.exe` they recognise nothing and stand down.

1. Copy the zip to Windows, e.g. from Ubuntu
   `cp dist/armada2-refit-*.zip /mnt/c/Users/<you>/Downloads/`, or open
   `\\wsl$\Ubuntu-24.04\home\<linux user>\armada2-refit\dist` in Explorer.
2. Unzip it, ideally into the game folder.
3. Double-click `install.bat`. Unzipped elsewhere, run it from cmd with the game's
   path: `install.bat "C:\GOG Games\Star Trek Armada II"`.
4. If the installer says it is missing, install Microsoft's **Visual C++ Redistributable
   (x86)**, which the widescreen patch needs.

The installer also puts in the ASI loader and the widescreen patch when they are
missing, and never overwrites a file that is already there. Leave optional layers out
with `install.bat -Without qol,rules`; undo everything with `install.bat -Uninstall`.
On Linux with Proton, the same zip's `./install.sh "/path/to/the game"` does the same
([`PLAYING.md`](PLAYING.md) covers the Wine DLL overrides).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `invalid /export` when linking | `lld-link` on `PATH` is Ubuntu's, not `rust-lld` | Redo the `lld-link` step; `command -v lld-link` must print `~/.local/bin/lld-link` |
| `can't find crate for core` / `the i686-pc-windows-msvc target may not be installed` | the target is missing, or the build runs a distribution `cargo` | `rustup target add i686-pc-windows-msvc`; `apt remove cargo rustc`; `export CARGO="$(command -v cargo)"` pointing at `~/.cargo/bin/cargo` |
| `llvm-dlltool`, `llvm-rc` or `llvm-readobj: command not found` | the `llvm` package is missing or too old | `sudo apt install llvm` on Ubuntu 24.04 |
| `linker 'cc' not found` | no host C compiler for the build scripts | `sudo apt install build-essential` |
| `$'\r': command not found`, `bad interpreter` | the clone has CRLF line endings | Clone again from the Ubuntu terminal into `~`, not with Git for Windows |
| the build is very slow | the checkout is on `/mnt/c` | Clone into the Linux home folder |
| the build is killed, or WSL stops responding | out of memory: WSL gets half the RAM by default | `export CARGO_BUILD_JOBS=2` before building |
| the game runs, but nothing changed | not GOG 1.1, or the ASI loader is not loading | Read the plugins' logs in `%LOCALAPPDATA%\armada2-refit\logs\` ([`PLAYING.md`](PLAYING.md)) |
