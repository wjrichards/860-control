#!/usr/bin/env python3
"""Task 1.2 spike 3: wave speed byte and ripple speed on the G512.

Wave block = 00*6, speed, 00, direction 1, intensity 100 (byte 7 kept 0,
the only value accepted so far). Ripple compares its slowest and fastest
accepted periods directly. Fixed ~55 s sequence, 1 s Off before each step,
RAM-only. Usage: python3 -I spike3_wave_ripple.py
"""
import os, select, sys, time

SW = 0x0A
ZONE = 0
OFF, STATIC, WAVE, RIPPLE = 0, 1, 4, 6


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


def wave(speed):
    return [0] * 6 + [speed, 0, 1, 100]


def ripple(period):
    return [0x00, 0xFF, 0xFF, 0, period >> 8 & 0xFF, period & 0xFF]


STEPS = [
    ("1 Wave, speed byte 10", WAVE, wave(10), 7),
    ("2 Wave, speed byte 30 (device default)", WAVE, wave(30), 7),
    ("3 Wave, speed byte 100", WAVE, wave(100), 8),
    ("4 Wave, speed byte 250", WAVE, wave(250), 10),
    ("5 Ripple cyan, period 10 -- TYPE", RIPPLE, ripple(10), 7),
    ("6 Ripple cyan, period 1000 -- TYPE", RIPPLE, ripple(1000), 7),
    ("7 Static white (left on at the end)", STATIC, [0xFF, 0xFF, 0xFF, 0], 0),
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
