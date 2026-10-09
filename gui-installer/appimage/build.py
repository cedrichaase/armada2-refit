#!/usr/bin/env python3
"""Assemble the installer's AppDir from the system this runs on.   build.py APPDIR

Copies the running Python and its standard library, PyGObject and pycairo, GTK 4 and
libadwaita with every library they need, the GObject typelibs, the compiled GLib
schemas, the Adwaita icons and gdk-pixbuf's loaders into APPDIR/usr, then the
installer itself and the files beside this one. build.sh packs the result.

What is left to the host, on purpose: the C library and its companions, everything
that talks to the graphics driver (GL, EGL, Vulkan, DRM, GBM), and the display
protocol libraries (Wayland, X11, xcb) -- bundling those breaks the host's own Mesa.
The result runs on any distribution with a C library at least as new as the one it
was built on, so CI builds it on the oldest we want (Ubuntu 24.04).

Standard library only; needs ldd, glib-compile-schemas and gdk-pixbuf-query-loaders.
"""
import glob
import os
import re
import shutil
import subprocess
import sys
import sysconfig

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, '..', 'armada2-refit-installer.py')

# Libraries the host provides. Matched against the soname.
EXCLUDE = re.compile(r'^(ld-linux.*|linux-vdso.*|libc\.so.*|libm\.so.*|libdl\.so.*|'
                     r'libpthread\.so.*|librt\.so.*|libutil\.so.*|libresolv\.so.*|libnsl\.so.*|'
                     r'libcrypt\.so.*|libstdc\+\+\.so.*|libgcc_s\.so.*|'
                     r'libGL.*|libEGL.*|libOpenGL.*|libgbm.*|libdrm.*|libvulkan.*|'
                     r'libwayland-.*|libX.*|libxcb.*|libxshmfence.*|libxkbcommon.*|'
                     r'libsystemd.*|libudev.*|libselinux.*|libasound.*|libpulse.*|'
                     r'libnvidia.*|libcuda.*)$')
# Libraries GTK reaches through dlopen or typelibs rather than as a dependency.
SEED_NAMES = ['gtk-4', 'adwaita-1', 'pango-1.0', 'pangocairo-1.0', 'pangoft2-1.0',
              'gdk_pixbuf-2.0', 'graphene-1.0', 'gio-2.0', 'gobject-2.0', 'glib-2.0',
              'gmodule-2.0', 'cairo', 'cairo-gobject', 'freetype', 'harfbuzz', 'fontconfig',
              'girepository-1.0', 'girepository-2.0', 'appstream']
STDLIB_SKIP = ('test', 'tests', 'idlelib', 'tkinter', 'turtledemo', 'ensurepip', 'lib2to3',
               'pydoc_data', '__pycache__', 'site-packages', 'dist-packages', 'config-*',
               'turtle.py', 'distutils')


def run(*cmd, **kw):
    return subprocess.run(cmd, check=True, text=True, capture_output=True, **kw).stdout


def copy(src, dst):
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    shutil.copy2(src, dst, follow_symlinks=True)


def ldconfig():
    out = run('ldconfig', '-p')
    libs = {}
    for line in out.splitlines():
        m = re.match(r'\s*(\S+) \((.*?)\) => (\S+)', line)
        if m and ('x86-64' in m.group(2) or 'AArch64' in m.group(2) or 'libc6' == m.group(2)):
            libs.setdefault(m.group(1), m.group(3))
    return libs


def needed(path):
    """{soname: resolved path} for everything ldd says `path` loads."""
    out = subprocess.run(['ldd', path], text=True, capture_output=True).stdout
    deps = {}
    for line in out.splitlines():
        m = re.match(r'\s*(\S+) => (/\S+)', line)
        if m:
            deps[os.path.basename(m.group(1))] = m.group(2)
    return deps


def main(appdir):
    usr = os.path.join(appdir, 'usr')
    if os.path.isdir(appdir):
        shutil.rmtree(appdir)
    os.makedirs(os.path.join(usr, 'lib'))
    pyver = f'{sys.version_info.major}.{sys.version_info.minor}'

    # Python: the executable and the standard library, no tests and no tkinter.
    os.makedirs(os.path.join(usr, 'bin'))
    shutil.copy2(os.path.realpath(sys.executable), os.path.join(usr, 'bin', 'python3'))
    shutil.copytree(sysconfig.get_path('stdlib'), os.path.join(usr, 'lib', f'python{pyver}'),
                    ignore=shutil.ignore_patterns(*STDLIB_SKIP), symlinks=False)
    site = os.path.join(usr, 'lib', f'python{pyver}', 'site-packages')
    import cairo
    import gi
    for pkg in (gi, cairo):
        shutil.copytree(os.path.dirname(pkg.__file__), os.path.join(site, pkg.__name__),
                        ignore=shutil.ignore_patterns('__pycache__'))
    for extra in glob.glob(os.path.join(os.path.dirname(os.path.dirname(gi.__file__)),
                                        'pygobject*')) + \
            glob.glob(os.path.join(os.path.dirname(os.path.dirname(cairo.__file__)), 'pycairo*')):
        if os.path.isdir(extra):
            shutil.copytree(extra, os.path.join(site, os.path.basename(extra)))

    # gdk-pixbuf's loaders (SVG icons among them) and a cache AppRun rewrites at start.
    loaders = (glob.glob('/usr/lib*/gdk-pixbuf-2.0/*/loaders') +
               glob.glob('/usr/lib/*/gdk-pixbuf-2.0/*/loaders'))
    ldest = os.path.join(usr, 'lib', 'gdk-pixbuf-2.0', '2.10.0', 'loaders')
    if loaders:
        for so in glob.glob(os.path.join(loaders[0], '*.so')):
            copy(so, os.path.join(ldest, os.path.basename(so)))

    # Every library: ldd of each seed, less what the host provides.
    cache = ldconfig()
    seeds = [os.path.join(usr, 'bin', 'python3')]
    for root, _, files in os.walk(os.path.join(usr, 'lib')):
        seeds += [os.path.join(root, f) for f in files if '.so' in f]
    for name in SEED_NAMES:
        seeds += [p for so, p in cache.items() if re.match(rf'lib{re.escape(name)}\.so\.\d+$', so)]
    bundled = {}
    for seed in seeds:
        for so, path in needed(seed).items():
            if not EXCLUDE.match(so):
                bundled[so] = path
        base = os.path.basename(seed)
        if base.startswith('lib') and not EXCLUDE.match(base) and not seed.startswith(usr):
            bundled[base] = seed
    # A seed found by name may itself need more: one more pass over what was added.
    for so, path in list(bundled.items()):
        for so2, path2 in needed(path).items():
            if not EXCLUDE.match(so2):
                bundled.setdefault(so2, path2)
    for so, path in sorted(bundled.items()):
        if os.path.basename(so).startswith('libpython'):
            continue
        copy(path, os.path.join(usr, 'lib', so))
    # libpython is a dependency of the executable on some distributions.
    for so, path in bundled.items():
        if so.startswith('libpython'):
            copy(path, os.path.join(usr, 'lib', so))

    # Typelibs.
    tdirs = [d for d in glob.glob('/usr/lib*/girepository-1.0') +
             glob.glob('/usr/lib/*/girepository-1.0') if os.path.exists(os.path.join(d, 'Gtk-4.0.typelib'))]
    if not tdirs:
        sys.exit('no Gtk-4.0.typelib found: install the GTK 4 introspection data')
    for t in glob.glob(os.path.join(tdirs[0], '*.typelib')):
        copy(t, os.path.join(usr, 'lib', 'girepository-1.0', os.path.basename(t)))

    # Schemas, compiled.
    sdest = os.path.join(usr, 'share', 'glib-2.0', 'schemas')
    os.makedirs(sdest)
    for x in glob.glob('/usr/share/glib-2.0/schemas/*.xml') + \
            glob.glob('/usr/share/glib-2.0/schemas/*.override'):
        shutil.copy2(x, sdest)
    run('glib-compile-schemas', sdest)

    # Icons: Adwaita carries every symbolic icon libadwaita and GTK ask for.
    for theme in ('Adwaita', 'hicolor'):
        src = os.path.join('/usr/share/icons', theme)
        if os.path.isdir(src):
            shutil.copytree(src, os.path.join(usr, 'share', 'icons', theme),
                            ignore=shutil.ignore_patterns('cursors'))

    # The pixbuf cache, with the bundle's own path left as a placeholder.
    if os.path.isdir(ldest):
        query = shutil.which('gdk-pixbuf-query-loaders') or next(
            iter(glob.glob('/usr/lib*/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders') +
                 glob.glob('/usr/lib/*/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders')), None)
        if query:
            out = run(query, *glob.glob(os.path.join(ldest, '*.so')))
            with open(os.path.join(usr, 'lib', 'gdk-pixbuf-loaders.cache.in'), 'w') as f:
                f.write(out.replace(usr, '@APPDIR@'))

    os.makedirs(os.path.join(usr, 'lib', 'gio', 'modules'))

    # The installer, and what appimagetool wants at the root.
    copy(SCRIPT, os.path.join(usr, 'share', 'armada2-refit', 'armada2-refit-installer.py'))
    copy(os.path.join(HERE, 'AppRun'), os.path.join(appdir, 'AppRun'))
    os.chmod(os.path.join(appdir, 'AppRun'), 0o755)
    for f in ('armada2-refit.desktop', 'armada2-refit.svg'):
        copy(os.path.join(HERE, f), os.path.join(appdir, f))
    copy(os.path.join(HERE, 'armada2-refit.desktop'),
         os.path.join(usr, 'share', 'applications', 'armada2-refit.desktop'))
    copy(os.path.join(HERE, 'armada2-refit.svg'),
         os.path.join(usr, 'share', 'icons', 'hicolor', 'scalable', 'apps', 'armada2-refit.svg'))
    os.symlink('armada2-refit.svg', os.path.join(appdir, '.DirIcon'))

    size = int(run('du', '-sk', appdir).split()[0]) // 1024
    print(f'AppDir {appdir}: {size} MB, {len(bundled)} libraries bundled')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(os.path.abspath(sys.argv[1]))
