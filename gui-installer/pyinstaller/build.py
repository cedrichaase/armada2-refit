#!/usr/bin/env python3
"""Freeze the GUI installer with PyInstaller, for the system this runs on.

    build.py [--out DIR] [--arch universal2|arm64|x86_64] [--no-icon]

  Windows  DIR/armada2-refit-installer-windows.exe: one file, windowed (no console), with
           its icon and a version resource carrying INSTALLER_VERSION
  macOS    DIR/armada2-refit-installer-macos.zip: "Armada2 Refit Installer.app", windowed,
           ad-hoc signed, zipped with ditto; --arch universal2 needs a universal2 Python
           (python.org's, which is what actions/setup-python installs on macOS)
  Linux    DIR/armada2-refit-installer-linux: one file -- only to try the build and the
           frozen program here; Linux releases the AppImage (appimage/build.sh)

The release names are read out of INSTALLER_ASSETS in the script, so they cannot drift
from what the installer looks for in a release. PyInstaller must be the version pinned in
requirements.txt (CI pip-installs that file). DIR defaults to a fresh temporary folder,
printed at the end; everything else is built in a temporary folder and removed.

The frozen program is Python too: `<exe> script.py args` runs the script (run_python in
the installer), which is how the release's install script runs the SOD rewrites with no
Python on the machine (A2_PYTHON). Those import only the standard library; the modules
they use are named below as hidden imports so they stay in even if the installer stops
importing one of them itself. It runs in Python's UTF-8 mode, so files and pipes are
UTF-8 on Windows as they are on Linux and macOS.
"""
import argparse
import hashlib
import os
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
GUI = os.path.dirname(HERE)
SCRIPT = os.path.join(GUI, 'armada2-refit-installer.py')
SVG = os.path.join(GUI, 'appimage', 'armada2-refit.svg')
APP_NAME = 'Armada2 Refit Installer'
BUNDLE_ID = 'io.github.cedrichaase.Armada2Refit.Installer'
SYSTEM = {'win32': 'windows', 'darwin': 'macos'}.get(sys.platform, 'linux')
# What the SOD scripts (models/*.py) and a2env.py import, and runpy for run_python. Tk is
# for the window on Windows and macOS.
HIDDEN = ['argparse', 'glob', 'hashlib', 'math', 'os', 're', 'shutil', 'struct', 'sys', 'runpy',
          'tkinter', 'tkinter.ttk', 'tkinter.filedialog', 'tkinter.messagebox']

sys.dont_write_bytecode = True     # no __pycache__ in the checkout
sys.path.insert(0, HERE)
import icon  # noqa: E402


def die(msg):
    print(f'build.py: {msg}', file=sys.stderr)
    sys.exit(1)


def pinned():
    with open(os.path.join(HERE, 'requirements.txt')) as f:
        m = re.search(r'^pyinstaller==(\S+)', f.read(), re.M | re.I)
    if not m:
        die('requirements.txt pins no pyinstaller')
    try:
        import PyInstaller
    except ImportError:
        die(f'PyInstaller is not installed: pip install -r {os.path.join(HERE, "requirements.txt")}')
    if PyInstaller.__version__ != m.group(1):
        die(f'PyInstaller {PyInstaller.__version__}; requirements.txt pins {m.group(1)}')
    return m.group(1)


def installer_version():
    r = subprocess.run([sys.executable, SCRIPT, '--check-version', '-'], capture_output=True,
                       text=True)
    v = r.stdout.strip()
    if not re.fullmatch(r'\d+\.\d+\.\d+', v):
        die(f'--check-version printed {v!r} ({r.stderr.strip()})')
    return v


def asset_names():
    with open(SCRIPT, encoding='utf-8') as f:
        src = f.read()
    m = re.search(r'^INSTALLER_ASSETS = \{(.*?)\}', src, re.M | re.S)
    names = dict(re.findall(r"'(\w+)': '([^']+)'", m.group(1))) if m else {}
    for s in ('windows', 'macos'):
        if s not in names:
            die(f'INSTALLER_ASSETS in the script names no {s} asset')
    names['linux'] = 'armada2-refit-installer-linux'      # not a release asset
    return names


def version_file(path, version, exe):
    """PyInstaller's version-resource spec (its VSVersionInfo, as Python literals)."""
    n = tuple(int(x) for x in version.split('.')) + (0,)
    strings = [('CompanyName', 'Armada II Refit'),
               ('FileDescription', 'Armada II Refit installer'),
               ('FileVersion', version), ('InternalName', 'armada2-refit-installer'),
               ('LegalCopyright', 'MIT License, Copyright (c) 2026 Cedric Haase'),
               ('OriginalFilename', exe), ('ProductName', 'Armada II Refit installer'),
               ('ProductVersion', version)]
    with open(path, 'w', encoding='utf-8') as f:
        f.write(f"""VSVersionInfo(
  ffi=FixedFileInfo(filevers={n}, prodvers={n}, mask=0x3f, flags=0x0, OS=0x40004,
                    fileType=0x1, subtype=0x0, date=(0, 0)),
  kids=[
    StringFileInfo([StringTable('040904B0', [
{chr(10).join(f'      StringStruct({k!r}, {v!r}),' for k, v in strings)}
    ])]),
    VarFileInfo([VarStruct('Translation', [1033, 1200])])
  ]
)
""")


def sha256(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for b in iter(lambda: f.read(1 << 20), b''):
            h.update(b)
    return h.hexdigest()


def run(cmd):
    print('+ ' + ' '.join(cmd), flush=True)
    subprocess.run(cmd, check=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--out', help='where the result goes (default: a new temporary folder)')
    ap.add_argument('--arch', choices=('universal2', 'arm64', 'x86_64'),
                    help='macOS only: the architectures of the .app (default: this Python\'s)')
    ap.add_argument('--no-icon', action='store_true', help='leave PyInstaller\'s icon out too')
    a = ap.parse_args()

    pin = pinned()
    version = installer_version()
    names = asset_names()
    out = os.path.abspath(a.out or os.path.join(tempfile.mkdtemp(), 'dist'))
    os.makedirs(out, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix='a2-pyinstaller-')
    try:
        dist, work = os.path.join(tmp, 'dist'), os.path.join(tmp, 'work')
        cmd = [sys.executable, '-m', 'PyInstaller', '--noconfirm', '--clean', '--log-level', 'WARN',
               '--distpath', dist, '--workpath', work, '--specpath', work,
               '--windowed', '--python-option', 'X utf8']
        for m in HIDDEN:
            cmd += ['--hidden-import', m]
        if SYSTEM == 'macos':
            if a.arch == 'universal2':
                archs = subprocess.run(['lipo', '-archs', sys.executable], capture_output=True,
                                       text=True).stdout.split()
                if not {'arm64', 'x86_64'} <= set(archs):
                    die(f'{sys.executable} is {" ".join(archs) or "not a Mach-O"}: a universal2 '
                        'build needs a universal2 Python (python.org\'s installer)')
            if a.arch:
                cmd += ['--target-arch', a.arch]
            cmd += ['--onedir', '--name', APP_NAME, '--osx-bundle-identifier', BUNDLE_ID]
        else:
            name = names[SYSTEM][:-4] if names[SYSTEM].endswith('.exe') else names[SYSTEM]
            cmd += ['--onefile', '--name', name]
        if a.no_icon:
            cmd += ['--icon', 'NONE']
        elif SYSTEM in ('windows', 'macos'):
            ic = os.path.join(tmp, 'icon.ico' if SYSTEM == 'windows' else 'icon.icns')
            icon.main([SVG, ic])
            cmd += ['--icon', ic]
        if SYSTEM == 'windows':
            vf = os.path.join(tmp, 'version.txt')
            version_file(vf, version, names['windows'])
            cmd += ['--version-file', vf]
        run(cmd + [SCRIPT])

        if SYSTEM == 'macos':
            app = os.path.join(dist, APP_NAME + '.app')
            plist = os.path.join(app, 'Contents', 'Info.plist')
            with open(plist, 'rb') as f:
                info = plistlib.load(f)
            info.update(CFBundleName=APP_NAME, CFBundleDisplayName='Armada II Refit Installer',
                        CFBundleIdentifier=BUNDLE_ID, CFBundleShortVersionString=version,
                        CFBundleVersion=version, NSHighResolutionCapable=True,
                        NSHumanReadableCopyright='MIT License, Copyright (c) 2026 Cedric Haase')
            with open(plist, 'wb') as f:
                plistlib.dump(info, f)
            # Info.plist changed after PyInstaller signed: sign the whole bundle again,
            # ad hoc. Not notarised -- the first launch needs Open from the context menu.
            run(['codesign', '--force', '--deep', '-s', '-', app])
            run(['codesign', '--verify', '--deep', '--strict', '--verbose=2', app])
            binary = os.path.join(app, 'Contents', 'MacOS', APP_NAME)
            print('architectures: ' + subprocess.run(['lipo', '-archs', binary], capture_output=True,
                                                     text=True).stdout.strip())
            result = os.path.join(out, names['macos'])
            if os.path.exists(result):
                os.unlink(result)
            run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', app, result])
        else:
            src = os.path.join(dist, names[SYSTEM])
            result = os.path.join(out, names[SYSTEM])
            shutil.copy2(src, result)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    print(f'{result}\n  installer {version}, PyInstaller {pin}, Python {sys.version.split()[0]}, '
          f'{os.path.getsize(result) / 1e6:.1f} MB, sha256 {sha256(result)}')


if __name__ == '__main__':
    main()
