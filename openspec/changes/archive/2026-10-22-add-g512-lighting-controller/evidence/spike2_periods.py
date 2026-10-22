#!/usr/bin/env python3
"""Task 1.2 spike 2: wave and ripple period range on the G512.

Uses the layouts confirmed in set-zone-effect.md. Fixed ~70 s sequence, each
step preceded by 1 s of Off. RAM-only (persistence 0); a replug restores the
keyboard's default lighting. Only Root.getFeature and ColorLedEffects fn3 are
called. Usage: python3 -I spike2_periods.py
"""
import os, select, sys, time

SW = 0x0A
ZONE = 0
OFF, STATIC, BREATHE, WAVE, RIPPLE = 0, 1, 2, 4, 6


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
            return "timeout", msg
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx:
            return f"HID++ error {rep[5]:#04x}", msg
        if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
            return "ok " + rep[4:20].hex(" "), msg


def be16(v):
    return [v >> 8 & 0xFF, v & 0xFF]


def wave(period):
    return [0] * 6 + be16(period) + [1, 100]


def ripple(period):
    return [0x00, 0xFF, 0xFF, 0] + be16(period)


STEPS = [
    ("1 Wave, period 1000", WAVE, wave(1000), 7),
    ("2 Wave, period 4000", WAVE, wave(4000), 8),
    ("3 Wave, period 12000", WAVE, wave(12000), 10),
    ("4 Wave, period 200 (high byte 0)", WAVE, wave(200), 6),
    ("5 Ripple cyan, period 10 -- TYPE", RIPPLE, ripple(10), 7),
    ("6 Ripple cyan, period 100 -- TYPE", RIPPLE, ripple(100), 7),
    ("7 Ripple cyan, period 1000 -- TYPE", RIPPLE, ripple(1000), 7),
    ("8 Breathe white, period 8000", BREATHE, [0xFF, 0xFF, 0xFF] + be16(8000) + [0, 100], 10),
    ("9 Static white (left on at the end)", STATIC, [0xFF, 0xFF, 0xFF, 0], 0),
]


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else find_dev()
    fd = os.open(path, os.O_RDWR)
    slot = call(fd, 0x00, 0, b"\x80\x70")[0]
    if not slot.startswith("ok") or slot.split()[1] == "00":
        sys.exit(f"ColorLedEffects not found: {slot}")
    idx = int(slot.split()[1], 16)
    print(f"device {path}, ColorLedEffects slot {idx:#04x}\n")
    for label, effect, block, secs in STEPS:
        call(fd, idx, 3, bytes([ZONE, OFF] + [0] * 11))
        time.sleep(1)
        params = bytes([ZONE, effect] + (block + [0] * 10)[:10] + [0])
        result, msg = call(fd, idx, 3, params)
        print(f"[{label}]\n  sent  {msg.hex(' ')}\n  reply {result}")
        sys.stdout.flush()
        time.sleep(secs)
    os.close(fd)
    print("\ndone")


if __name__ == "__main__":
    main()
