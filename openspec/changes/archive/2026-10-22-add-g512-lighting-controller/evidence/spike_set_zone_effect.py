#!/usr/bin/env python3
"""Task 1.2 spike: try ColorLedEffects (0x8070) setZoneEffect layouts on the G512.

Runs a fixed ~75 s sequence so the person watching the keyboard can match what
they see to each step. Each step is preceded by 1 s of Off as a separator.
Every write is RAM-only (persistence byte 0); unplugging the keyboard restores
its own default lighting.

Candidate layout (from Solaar/libratbag, unverified):
  params = zone, effect_index, block[10], persistence
  Static  block = R G B ramp
  Breathe block = R G B period_hi period_lo form intensity
  Cycle   block = 0 0 0 0 0 period_hi period_lo intensity
  Ripple  block = R G B 0 period_hi period_lo
  Wave    unknown: two guesses (A = Cycle-shaped, B = g810-led-shaped)

Only Root.getFeature and ColorLedEffects (fn0 getInfo is not used; fn3
setZoneEffect) are called. Usage: python3 -I spike_set_zone_effect.py
(works without sudo once packaging/70-g512-control.rules is installed).
"""
import os, select, sys, time

SW = 0x0A
ZONE = 0
# Effect *indexes* in zone 0 as reported by hidpp_effects.py (not effect IDs).
OFF, STATIC, BREATHE, CYCLE, WAVE, RIPPLE = 0, 1, 2, 3, 4, 6


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


def rgb(hexstr):
    return list(bytes.fromhex(hexstr))


STEPS = [
    # (label, effect_index, block, seconds)
    ("1  Static red #ff0000", STATIC, rgb("ff0000") + [0], 5),
    ("2  Static blue #0000ff (checks R,G,B byte order)", STATIC, rgb("0000ff") + [0], 4),
    ("3  Breathe magenta, period 992", BREATHE, rgb("ff00ff") + be16(992) + [0, 100], 6),
    ("4  Breathe magenta, period 4000", BREATHE, rgb("ff00ff") + be16(4000) + [0, 100], 8),
    ("5  Cycle, period 992", CYCLE, [0] * 5 + be16(992) + [100], 6),
    ("6  Cycle, period 8000", CYCLE, [0] * 5 + be16(8000) + [100], 6),
    ("7  Wave guess A (Cycle-shaped), period 30", WAVE, [0] * 5 + be16(30) + [100], 6),
    ("8  Wave guess A (Cycle-shaped), period 120", WAVE, [0] * 5 + be16(120) + [100], 6),
    ("9  Wave guess B (g810-shaped), period 30, direction 1", WAVE, [0] * 6 + [30, 0, 1, 100], 6),
    ("10 Ripple cyan, period 30 -- TYPE ON THE KEYBOARD", RIPPLE, rgb("00ffff") + [0] + be16(30), 8),
    ("11 Off", OFF, [], 3),
    ("12 Static white #ffffff (left on at the end)", STATIC, rgb("ffffff") + [0], 0),
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
        if effect != OFF:
            call(fd, idx, 3, bytes([ZONE, OFF] + [0] * 10 + [0]))
            time.sleep(1)
        params = bytes([ZONE, effect] + (block + [0] * 10)[:10] + [0])
        result, msg = call(fd, idx, 3, params)
        print(f"[{label}]\n  sent  {msg.hex(' ')}\n  reply {result}")
        sys.stdout.flush()
        time.sleep(secs)
    os.close(fd)
    print("\ndone - unplug/replug the keyboard to return to its default lighting")


if __name__ == "__main__":
    main()
