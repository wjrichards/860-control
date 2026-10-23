#!/usr/bin/env python3
"""Spike 6a: read-only survey of the G502 HERO (046d:c08b), plus a backup of
its onboard-profile memory.

Only read functions are called:
  Root (0x0000) fn0 getFeature
  ColorLedEffects (0x8070) fn0 getInfo, fn1 getZoneInfo, fn2 getZoneEffectInfo
  AdjustableDpi (0x2201) fn0 getSensorCount, fn1 getSensorDpiList, fn2 getSensorDpi
  AdjustableReportRate (0x8060) fn0 getReportRateList, fn1 getReportRate
  OnboardProfiles (0x8100) fn0 getInfo, fn2 getMode, fn4 getCurrentProfile,
                           fn5 memoryRead, fn11 getCurrentDpiIndex
Nothing is set, and no memory is written.

Usage (prints to stdout; tee keeps a copy owned by you):
  sudo python3 -I spike6_mouse_read.py /dev/hidrawN | tee spike6-read.txt
"""
import os, select, sys

SW = 0x0B


def find_dev():
    for n in sorted(os.listdir("/sys/class/hidraw")):
        ue = open(f"/sys/class/hidraw/{n}/device/uevent").read()
        rd = open(f"/sys/class/hidraw/{n}/device/report_descriptor", "rb").read()
        if "046D:0000C08B" in ue.upper() and b"\x85\x11" in rd:
            return f"/dev/{n}"
    sys.exit("G502 HERO HID++ interface not found")


class Err(Exception):
    pass


def call(fd, feat_idx, func, params=b""):
    msg = bytes([0x11, 0xFF, feat_idx, (func << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 1.0)
        if not r:
            raise Err(f"timeout (feature slot {feat_idx:#04x} fn{func})")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx and rep[4] == ((func << 4) | SW):
            raise Err(f"HID++ error {rep[5]:#04x}")
        if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
            return rep[4:20]


def slot(fd, feature):
    r = call(fd, 0x00, 0, bytes([feature >> 8, feature & 0xFF]))
    return r[0] or None


def hx(b):
    return b.hex(" ")


def u16(b, i):
    return b[i] << 8 | b[i + 1]


def crc_ccitt(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else crc << 1
            crc &= 0xFFFF
    return crc


def lighting(fd, idx):
    print(f"\n== ColorLedEffects (0x8070) at slot {idx:#04x}")
    info = call(fd, idx, 0)
    print(f"getInfo: {hx(info)}  zones={info[0]}")
    for z in range(info[0]):
        zi = call(fd, idx, 1, bytes([z]))
        print(f"  zone {z}: raw {hx(zi)}  location={u16(zi, 1):#06x}  effects={zi[3]}")
        for e in range(zi[3]):
            ei = call(fd, idx, 2, bytes([z, e]))
            print(f"    #{e} id={u16(ei, 2):#06x} caps={u16(ei, 4):#06x} period={u16(ei, 6)}  raw {hx(ei)}")


def dpi(fd, idx):
    print(f"\n== AdjustableDpi (0x2201) at slot {idx:#04x}")
    count = call(fd, idx, 0)[0]
    print(f"sensors={count}")
    for s in range(count):
        print(f"  sensor {s} dpi list raw: {hx(call(fd, idx, 1, bytes([s])))}")
        cur = call(fd, idx, 2, bytes([s]))
        print(f"  sensor {s} current raw: {hx(cur)}  current={u16(cur, 1)} default={u16(cur, 3)}")


def report_rate(fd, idx):
    print(f"\n== AdjustableReportRate (0x8060) at slot {idx:#04x}")
    lst = call(fd, idx, 0)
    rates = [ms for ms in range(1, 9) if lst[0] & (1 << (ms - 1))]
    print(f"list raw: {hx(lst)}  supported periods (ms): {rates}")
    cur = call(fd, idx, 1)
    print(f"current raw: {hx(cur)}  period={cur[0]} ms")


def profiles(fd, idx):
    print(f"\n== OnboardProfiles (0x8100) at slot {idx:#04x}")
    info = call(fd, idx, 0)
    names = ["memory_model", "profile_format", "macro_format", "profile_count", "profile_count_oob",
             "button_count", "sector_count"]
    fields = {n: info[i] for i, n in enumerate(names)}
    fields["sector_size"] = u16(info, 7)
    fields["mechanical_layout"] = info[9]
    fields["various_info"] = info[10]
    print(f"getInfo raw: {hx(info)}")
    for k, v in fields.items():
        print(f"  {k} = {v} ({v:#x})")
    for label, fn in (("getMode (1=onboard, 2=host)", 2), ("getCurrentProfile", 4), ("getCurrentDpiIndex", 11)):
        try:
            print(f"{label}: {hx(call(fd, idx, fn))}")
        except Err as e:
            print(f"{label}: {e}")

    size = fields["sector_size"]
    offsets = list(range(0, size - 15, 16))
    if size % 16:
        offsets.append(size - 16)
    sectors = list(range(fields["sector_count"])) + [0x100 + i for i in range(fields["profile_count_oob"])]
    print(f"\n== Memory backup: {len(sectors)} sectors of {size} bytes "
          f"(user 0x0000..{fields['sector_count'] - 1:#06x}, ROM 0x0100..)")
    for sector in sectors:
        data = bytearray(size)
        try:
            for off in offsets:
                chunk = call(fd, idx, 5, bytes([sector >> 8, sector & 0xFF, off >> 8, off & 0xFF]))
                data[off:off + 16] = chunk
        except Err as e:
            print(f"sector {sector:#06x}: unreadable ({e})")
            continue
        stored = u16(data, size - 2)
        computed = crc_ccitt(data[:size - 2])
        blank = all(b == 0xFF for b in data)
        state = "blank (all ff)" if blank else ("crc ok" if stored == computed else
                                                 f"crc MISMATCH stored {stored:#06x} computed {computed:#06x}")
        print(f"sector {sector:#06x}: {state}")
        print(f"  {data.hex()}")


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else find_dev()
    fd = os.open(path, os.O_RDWR)
    print(f"device: {path}")
    found = {f: slot(fd, f) for f in (0x8070, 0x2201, 0x8060, 0x8100)}
    print("feature slots: " + ", ".join(f"{f:#06x}={s:#04x}" if s else f"{f:#06x}=absent" for f, s in found.items()))
    for feature, fn in ((0x8070, lighting), (0x2201, dpi), (0x8060, report_rate), (0x8100, profiles)):
        if found[feature]:
            try:
                fn(fd, found[feature])
            except Err as e:
                print(f"  stopped: {e}")
    os.close(fd)


if __name__ == "__main__":
    main()
