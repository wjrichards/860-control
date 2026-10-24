#!/usr/bin/env python3
"""Spike 6b: which zone is which on the G502 HERO, and does lighting set
from the computer hold while the mouse stays in onboard mode?

Calls only:
  Root (0x0000) fn0 getFeature
  ColorLedEffects (0x8070) fn1 getZoneInfo, fn2 getZoneEffectInfo,
                           fn3 setZoneEffect with persistence 0 (RAM only)
  OnboardProfiles (0x8100) fn2 getMode (read only; the mode is never changed)

A fixed ~50 s sequence; watch the mouse. Unplugging and replugging it brings
back the lighting from its stored profile.
Usage: sudo python3 -I spike6_mouse_light.py /dev/hidrawN | tee spike6-light.txt
"""
import os, select, sys, time

SW = 0x0C
STATIC_ID, OFF_ID = 0x0001, 0x0000


def call(fd, feat_idx, func, params=b""):
    msg = bytes([0x11, 0xFF, feat_idx, (func << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 1.0)
        if not r:
            return None, "timeout"
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx and rep[4] == ((func << 4) | SW):
            return None, f"HID++ error {rep[5]:#04x}"
        if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
            return rep[4:20], "ok"


def slot(fd, feature):
    r, _ = call(fd, 0x00, 0, bytes([feature >> 8, feature & 0xFF]))
    return r[0] if r else None


def effect_indexes(fd, idx, zone):
    """Effect index of Static and Off in this zone, looked up by effect ID."""
    zi, _ = call(fd, idx, 1, bytes([zone]))
    found = {}
    for e in range(zi[3]):
        ei, _ = call(fd, idx, 2, bytes([zone, e]))
        found.setdefault(ei[2] << 8 | ei[3], e)
    return found[STATIC_ID], found[OFF_ID]


def main():
    path = sys.argv[1]
    fd = os.open(path, os.O_RDWR)
    leds, prof = slot(fd, 0x8070), slot(fd, 0x8100)
    zones = {z: effect_indexes(fd, leds, z) for z in (0, 1)}
    t0 = time.monotonic()

    def mode():
        r, status = call(fd, prof, 2)
        return f"{r[0]}" if r else status

    def show(label, colors, secs):
        for z, rgb in colors.items():
            static, off = zones[z]
            block = list(bytes.fromhex(rgb)) + [0] * 7 if rgb else [0] * 10
            params = bytes([z, static if rgb else off] + block + [0])
            _, status = call(fd, leds, 3, params)
            print(f"[{time.monotonic() - t0:5.1f}s] zone {z} -> {rgb or 'off'}: {status}")
        print(f"         {label}  (mode {mode()})")
        sys.stdout.flush()
        time.sleep(secs)

    print(f"device {path}: ColorLedEffects slot {leds:#04x}, OnboardProfiles slot {prof:#04x}")
    print(f"effect indexes (static, off): {zones}; mode at start: {mode()}\n")
    show("STEP 1: zone 0 RED, zone 1 off -- which part is red?", {0: "ff0000", 1: None}, 6)
    show("STEP 2: zone 0 off, zone 1 BLUE -- which part is blue?", {0: None, 1: "0000ff"}, 6)
    show("STEP 3: both GREEN", {0: "00ff00", 1: "00ff00"}, 6)
    show("STEP 4: both WHITE -- press the DPI-up button once now, then watch for 20 s",
         {0: "ffffff", 1: "ffffff"}, 20)
    print(f"[{time.monotonic() - t0:5.1f}s] done, mode {mode()}. Still white? "
          "Unplug and replug the mouse to get its own lighting back.")
    os.close(fd)


if __name__ == "__main__":
    main()
