#!/usr/bin/env python3
"""Read-only probe of the G512's lighting features (HID++ 2.0 0x8070 / 0x8080).

Calls getter functions only:
  0x8070 ColorLedEffects  fn0 getInfo, fn1 getZoneInfo, fn2 getZoneEffectInfo
  0x8080 PerKeyLighting   fn0 getInfo(type, page) over a small grid
Never calls setZoneEffect (fn3), any 0x8080 setter or frame commit, or any
feature other than Root/FeatureSet. Feature slots are looked up, not hardcoded.
Usage: sudo python3 -I hidpp_effects.py [/dev/hidrawN]
"""
import os, select, sys

SW = 0x0A
# Effect IDs as named by Solaar/libratbag; unconfirmed for this device.
EFFECTS = {
    0x0000: "Disabled", 0x0001: "Static", 0x0002: "Pulse", 0x0003: "Cycle",
    0x0008: "Boot", 0x0009: "Demo", 0x000A: "Breathe", 0x000B: "Ripple",
}
# Zone location IDs as named by Solaar; unconfirmed for this device.
LOCATIONS = {0x0001: "Primary", 0x0002: "Logo", 0x0003: "LeftSide", 0x0004: "RightSide"}


class HidppError(Exception):
    pass


def find_dev():
    for n in sorted(os.listdir("/sys/class/hidraw")):
        ue = open(f"/sys/class/hidraw/{n}/device/uevent").read()
        rd = open(f"/sys/class/hidraw/{n}/device/report_descriptor", "rb").read()
        if "046D:0000C33C" in ue.upper() and b"\x06\x43\xff" in rd:
            return f"/dev/{n}"
    sys.exit("G512 HID++ interface not found")


def call(fd, feat_idx, func, params=b""):
    msg = bytes([0x11, 0xFF, feat_idx, (func << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 1.0)
        if not r:
            raise TimeoutError(f"no reply to feat {feat_idx:#x} func {func}")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx:
            raise HidppError(f"HID++ error {rep[5]:#04x}")
        if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
            return rep[4:]


def bits(v, width=16):
    return ",".join(str(b) for b in range(width) if v >> b & 1) or "none"


def u16(b, i):
    return (b[i] << 8) | b[i + 1]


def probe_effects(fd, idx):
    print(f"== ColorLedEffects (0x8070) at slot {idx:#04x}")
    info = call(fd, idx, 0)
    zones, nv, ext = info[0], u16(info, 1), u16(info, 3)
    print(f"getInfo raw: {info[:16].hex(' ')}")
    print(f"  zones={zones}  nvCapabilities={nv:#06x} (bits {bits(nv)})"
          f"  extCapabilities={ext:#06x} (bits {bits(ext)})")
    for z in range(zones):
        zi = call(fd, idx, 1, bytes([z]))
        loc, count = u16(zi, 1), zi[3]
        print(f"\n  zone {z}: raw {zi[:16].hex(' ')}")
        print(f"    location={loc:#06x} ({LOCATIONS.get(loc, '?')})  effects={count}")
        print(f"    {'#':>3}  {'id':>6}  {'caps':>6}  {'period':>6}  name / raw")
        for e in range(count):
            try:
                ei = call(fd, idx, 2, bytes([z, e]))
            except HidppError as err:
                print(f"    {e:>3}  {err}")
                continue
            eid, caps, period = u16(ei, 2), u16(ei, 4), u16(ei, 6)
            print(f"    {e:>3}  {eid:#06x}  {caps:#06x}  {period:>6}  "
                  f"{EFFECTS.get(eid, '?'):<9} {ei[:16].hex(' ')}")


def probe_perkey(fd, idx):
    print(f"\n== PerKeyLighting (0x8080) at slot {idx:#04x}: getInfo(type, page)")
    for t in range(4):
        for page in range(4):
            try:
                r = call(fd, idx, 0, bytes([t, page]))
            except (HidppError, TimeoutError) as err:
                print(f"  type={t} page={page}: {err}")
                break  # stop paging this type once it errors
            print(f"  type={t} page={page}: {r[:16].hex(' ')}")
            if t == 0 and r[0] == 0:
                ids = [8 * (i - 2) + b for i in range(2, 16) for b in range(8) if r[i] >> b & 1]
                print(f"    if bytes 2.. are a key-ID bitmap: {len(ids)} IDs set, first {ids[:12]}")


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else find_dev()
    fd = os.open(path, os.O_RDWR)
    print(f"device: {path}\n")
    slots = {}
    for fid in (0x8070, 0x8080):
        r = call(fd, 0x00, 0, bytes([fid >> 8, fid & 0xFF]))
        if r[0]:
            slots[fid] = r[0]
    if 0x8070 in slots:
        probe_effects(fd, slots[0x8070])
    if 0x8080 in slots:
        probe_perkey(fd, slots[0x8080])
    os.close(fd)


if __name__ == "__main__":
    main()
