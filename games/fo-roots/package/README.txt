Armada II Refit for Fleet Operations: Roots -- a test build
===========================================================

This is a preview of the Armada II Refit renderer, HUD and quality-of-life plugins running
on Roots. It is private, made to be tried out. Please don't pass it on.

It needs Roots 5.0Z11. The plugins recognise the game by its ArmadaL.exe, and on any other
build they switch off whatever they cannot verify. The installer warns you when it finds a
different build.


What it puts in Roots' Data\ folder
-----------------------------------

  d3d8.dll, Display8.ini   Display8: Direct3D 8 drawn through Vulkan, or through Direct3D 12
                           when Vulkan does not start. It takes the place of DxWrapper's
                           d3d8.dll, which is kept as d3d8.dll.a2display-backup.
                           dxwrapper.dll, dxwrapper.ini, d3d9.dll and ReShade are not touched.
  winmm.dll                the Ultimate ASI Loader (ThirteenAG, MIT), which loads the .asi
                           plugins below. A winmm.dll already there is kept as
                           winmm.dll.a2asi-backup.
  Render3D.asi, .ini       the 3D view: HDR, anti-aliasing, bloom, hulls lit by the scene
  Surfaces\, Nebulae\      with shadows, skies, effects, planets and nebulae. Its recipes
                           are the files in these two folders.
  UI.asi, UI.ini           the HUD panels Roots changes least, redrawn at any aspect ratio:
                           the resources, the top-right buttons, the pause box, the chat, the
                           selection, the frame-rate panel and the loading screen. The button
                           bar, the info panel, the minimap and the menus stay Roots' own.
  QOL.asi, QOL.ini         a faster right-drag pan; Shift+number adds the selection to a
                           group (Alt+number takes Shift's old job); move queues (hold Tab
                           and right-click points: the ships set off at the first one).
                           Everything else in it is off on Roots.

Nothing of the game's own files is changed, and the executable is patched in memory only.
Uninstalling takes all of it out and puts DxWrapper's d3d8.dll back.


Installing
----------

Windows: unzip this folder and run install.bat. It looks for Roots in the folder you
unzipped it into (the Roots folder, the one that holds Data\, or Data\ itself), then in
Programs and Features, and otherwise asks you to pick the folder. You can also pass it:

    install.bat "D:\Games\Fleet Operations Roots"

Linux or macOS (Wine, Proton, Heroic, Lutris):

    ./install.sh "/path/to/Fleet Operations Roots"

Under Wine, the game must load the native d3d8.dll and winmm.dll. Set
WINEDLLOVERRIDES="winmm=n,b;d3d8=n,b" (in Heroic: Settings > Advanced > Environment
variables).

Running it again updates the plugins. Your .ini files are kept: delete one and run it
again to get the shipped file back.


Uninstalling
------------

    install.bat -Uninstall            (Windows)
    ./install.sh --uninstall          (Linux, macOS)

This takes out every file the package put in and puts DxWrapper's d3d8.dll back. It leaves
Data\RefitPlayer.ini (your Refit screen settings), if there is one; delete it by hand.


In the game
-----------

Ctrl+F12 in a mission opens the Refit screen. It switches each part on or off (the 3D view's
modules, each HUD panel) and changes their settings while you play. Your changes are saved in
Data\RefitPlayer.ini. A part switched off gives that part back to the game's own drawing.

To choose the graphics backend, edit Data\Display8.ini:
  Backend=auto      Vulkan, then Direct3D 12 (the default)
  Backend=d3d12     Direct3D 12 first
  Backend=none      the game's own Direct3D 8 (DxWrapper's look), with every plugin off


Known gaps
----------

- The menus and the front end are Roots' own. Refit draws the missions and the loading
  screen only.
- Formation moves (command 11) draw no path line.
- The warp path, billboards and some special weapons have not been seen on Roots yet.


When something is wrong
-----------------------

The plugins write logs, rewritten at each launch, in

    %LOCALAPPDATA%\armada2-refit\logs\
    (under Wine: drive_c/users/<user>/AppData/Local/armada2-refit/logs/ in the prefix)

Display8.log, Render3D.log, UI.log and QOL.log. Please send them along with a screenshot
and a note of what you were doing. A crash also leaves Roots' own report in
Data\settings_saves_logs\Logs\exceptArmada2.txt.

If the game does not start at all, set Backend=none in Data\Display8.ini, or uninstall.

Armada II Refit is MIT-licensed (LICENSE). The ASI loader's licence is in licenses\.
