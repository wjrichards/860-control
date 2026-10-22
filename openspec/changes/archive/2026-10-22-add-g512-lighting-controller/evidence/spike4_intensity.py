#!/usr/bin/env python3
"""Spike 4: does the intensity byte dim Cycle and Wave on the G512?

Cycle block = 00*5 period_hi period_lo INTENSITY; Wave block =
00*6 speed 00 direction INTENSITY (set-zone-effect.md). Fixed ~30 s
sequence, 1 s Off before each step, RAM-only. Ends on Static white; a running
g512d re-applies its saved state on the next replug or resume.
Usage: python3 -I spike4_intensity.py
"""
import os, select, sys, time

SW = 0x0A
ZONE = 0
OFF, STATIC, CYCLE, WAVE = 0, 1, 3, 4


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


def cycle(intensity):
    return [0] * 5 + [0x0B, 0xB8, intensity]  # period 3000


def wave(intensity):
    return [0] * 6 + [30, 0, 1, intensity]


STEPS = [
    ("1 Cycle, intensity 100", CYCLE, cycle(100), 6),
    ("2 Cycle, intensity 20", CYCLE, cycle(20), 6),
    ("3 Wave, intensity 100", WAVE, wave(100), 6),
    ("4 Wave, intensity 20", WAVE, wave(20), 6),
    ("5 Static white (left on at the end)", STATIC, [0xFF, 0xFF, 0xFF, 0], 0),
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
