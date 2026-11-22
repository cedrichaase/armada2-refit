#!/usr/bin/env python3
"""fo-forms.py: reads a FleetOpsHook.dll once per new build for the UI's forms part
(PLAN-FLEETOPS.md, FO-6 step 7, option C, decision 147). Stdlib only; reads the files it
is given and writes nothing.

    tools/engine/fo-forms.py FleetOpsHook.dll [--map FleetOpsHook.map] [--label NAME]

What it says:

- the identity line to add to `FAMILY` in `ui/src/engine/forms.rs`: the CRC-32 of the
  file's `.text` as `crates/a2engine` computes it (`companion_crc`), and the file's sha256;
- for each form the UI draws, whether its class is in the file (found by the VCL's own
  class layout: a class pointer V whose self-pointer slot holds V and whose name slot
  names a counted string), and whether each control the UI reads is in its published
  field table, with the control's class;
- with `--map`, whether the map's names for the VCL accessors whose fields the UI reads
  are present (the map describes the unpatched build: names and addresses, not bytes).

Exit status 1 when a form or control the UI needs is missing: a FleetOpsHook that is not
in the family for the forms part, to be looked at before its line is added.
"""
import argparse
import hashlib
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe  # noqa: E402

# The VCL's class layout (Delphi 2007): slots before the class pointer. Not offsets into
# a Roots structure: the language runtime's own, the same in every Delphi 2007 module.
SELF_PTR = -0x4c    # not an offset into game memory
FIELD_TABLE = -0x38  # not an offset into game memory
CLASS_NAME = -0x2c   # not an offset into game memory
PARENT = -0x24       # not an offset into game memory

# The forms and the controls the UI reads or presses on them (by the names the forms give
# them, which are symbol names; ui/src/menus/forms/ names the same ones).
NEEDS = {
    'TMainForm': ['buttonSinglePlayer', 'buttonInstantAction', 'buttonMultiplayer', 'buttonOptions',
                  'buttonMapEdit', 'buttonCredits', 'buttonExit', 'labelVersion'],
    'TOptionsMenuForm': ['buttonBack', 'btnGraphicSettings', 'btnSoundSettings', 'btnGameSettings',
                         'btnShowModSettings', 'btnLoadSavegame', 'btnSaveGame', 'btnAdmiralsLog',
                         'btnEndMission', 'btnExitWindows'],
    'TYesNoForm': [],
    'TManualIPForm': [],
    'TMultiplayerConnectionForm': [],
    'TGraphicOptionsForm': ['buttonBack', 'comboDevicePrimary', 'comboDevicePrimaryResolutions'],
    'TAdvancedOptionsForm': [],
    'TMapSelectForm': [],
    # The map editor's Load Map / Save Map form, and the VCL's message box (its overwrite
    # question): the message box's controls are made at run time, so none is published.
    'TMapSaveForm': ['grdMaps', 'edtSaveName', 'btnConfirm', 'btnClose', 'lbl1'],
    'TMessageForm': [],
}

# Map names of the VCL accessors behind the private fields the UI reads (decision 147).
ACCESSORS = ['Controls.TControl.GetText', 'Controls.TControl.SetVisible', 'Controls.TControl.GetEnabled',
             'Controls.TWinControl.GetHandle', 'Classes.TComponent.GetComponent',
             'Classes.TComponent.GetComponentCount']


class Image:
    def __init__(self, data):
        self.exe = pe.Exe(data)
        self.data = data
        self.base = self.exe.image_base

    def off(self, va):
        rva = va - self.base
        for _name, sva, vsize, raw, rawptr, _f in self.exe.sections:
            if sva <= rva < sva + max(vsize, raw) and rva - sva < raw:
                return rawptr + rva - sva
        return None

    def u32(self, va):
        o = self.off(va)
        if o is None or o + 4 > len(self.data):
            return None
        return struct.unpack_from('<I', self.data, o)[0]

    def short(self, va):
        o = self.off(va)
        if o is None or o >= len(self.data):
            return None
        n = self.data[o]
        b = self.data[o + 1:o + 1 + n]
        try:
            return b.decode('ascii') if len(b) == n and n else None
        except UnicodeDecodeError:
            return None

    def classes(self):
        """{name: class pointer} for every class the VCL layout finds."""
        out = {}
        for name, sva, vsize, raw, rawptr, _f in self.exe.sections:
            for o in range(rawptr, rawptr + min(vsize, raw) - 4, 4):
                v = struct.unpack_from('<I', self.data, o)[0]
                here = self.base + sva + (o - rawptr)
                if v != here - SELF_PTR:
                    continue
                n = self.short(self.u32(v + CLASS_NAME) or 0)
                if n and n[0] == 'T' and n.isidentifier():
                    out.setdefault(n, v)
        return out

    def fields(self, vmt):
        """{field name: class name} of a class's published field table, its own and its parents'."""
        out = {}
        for _ in range(64):
            t = self.u32(vmt + FIELD_TABLE)
            if t:
                count = self.u32(t) & 0xffff
                ctab = self.u32(t + 2)
                at = t + 6
                for _i in range(count):
                    idx = (self.u32(at + 4) or 0) & 0xffff
                    name = self.short(at + 6)
                    if name is None:
                        break
                    cls = None
                    if ctab:
                        ref = self.u32(ctab + 2 + 4 * idx)
                        cv = self.u32(ref) if ref else None
                        cls = self.short(self.u32(cv + CLASS_NAME) or 0) if cv else None
                    out.setdefault(name, cls)
                    at += 6 + 1 + len(name)
            p = self.u32(vmt + PARENT)
            if not p:
                break
            vmt = self.u32(p)
            if not vmt:
                break
        return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('dll')
    ap.add_argument('--map')
    ap.add_argument('--label', default='')
    a = ap.parse_args()
    data = open(a.dll, 'rb').read()
    img = Image(data)
    crc = pe.section_crc(data, '.text')
    label = a.label or os.path.basename(os.path.dirname(os.path.abspath(a.dll)))
    print(f'identity: (0x{crc:08x}, "{label}"),  // sha256 {hashlib.sha256(data).hexdigest()}')
    classes = img.classes()
    print(f'classes found by the VCL layout: {len(classes)}')
    bad = 0
    for form, ctls in NEEDS.items():
        v = classes.get(form)
        if v is None:
            print(f'  {form}: MISSING')
            bad += 1
            continue
        f = img.fields(v)
        missing = [c for c in ctls if c not in f]
        bad += len(missing)
        print(f'  {form}: {len(f)} published fields' + (f'; missing {", ".join(missing)}' if missing else ''))
        for c in ctls:
            if c in f:
                print(f'    {c}: {f[c]}')
    if a.map:
        text = open(a.map, encoding='latin-1').read()
        for n in ACCESSORS:
            ok = n in text
            print(f'  map: {n} {"present" if ok else "MISSING"}')
            bad += not ok
    print('ok' if not bad else f'{bad} missing')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
