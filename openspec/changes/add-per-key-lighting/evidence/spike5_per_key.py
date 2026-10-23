#!/usr/bin/env python3
"""Spike 5: per-key lighting on the G512 (HID++ 0x8080 PerKeyLighting).

Packet layout from g810-led's G512/G513 path (src/classes/Keyboard.cpp):
  set keys  12 ff <slot> 3a  00 01 00 0e  then 14 x (key, R, G, B), 64 bytes
  commit    11 ff <slot> 5a                                          20 bytes
Key IDs are USB HID keyboard usages (0x04 A ... 0xe0 Left Ctrl). Only
0x8080 functions 3 and 5, and 0x8070 setZoneEffect (fn3, as confirmed in
v1), are called. Everything is RAM-only; g512d's SetState restores the
saved lighting afterwards.

The last step listens on the HID++ interface while the user presses the
Game Mode key. Only HID++ reports (IDs 0x11/0x12) are printed. Keyboard
and media reports (IDs 0x01/0x02) on the same interface are counted, never
printed, because they carry keystrokes.

Usage: python3 -I spike5_per_key.py [--dry-run] [/dev/hidrawN]
"""
import os, select, sys, time

SW = 0x0A
ZONE = 0
OFF, STATIC, WAVE = 0, 1, 4
LEFT_CTRL, ESC = 0xE0, 0x29
DRY = "--dry-run" in sys.argv

# ANSI 104: letters, digits, Enter..Backslash, ;..Caps, F1-F12,
# PrtSc..Up, NumLock..Num., Menu, modifiers.
KEYS = (list(range(0x04, 0x32)) + list(range(0x33, 0x53)) + list(range(0x53, 0x64))
        + [0x65] + list(range(0xE0, 0xE8)))
assert len(KEYS) == 104, len(KEYS)
RED, GREEN, BLUE, WHITE, AMBER = (255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255), (255, 120, 0)

T0 = time.monotonic()


def log(msg):
    print(f"[{time.monotonic() - T0:6.2f}s] {msg}")
    sys.stdout.flush()


def find_dev():
    for n in sorted(os.listdir("/sys/class/hidraw")):
        ue = open(f"/sys/class/hidraw/{n}/device/uevent").read()
        rd = open(f"/sys/class/hidraw/{n}/device/report_descriptor", "rb").read()
        if "046D:0000C33C" in ue.upper() and b"\x06\x43\xff" in rd:
            return f"/dev/{n}"
    sys.exit("G512 HID++ interface not found")


class Dev:
    def __init__(self, path):
        self.fd = None if DRY else os.open(path, os.O_RDWR)
        self.sent = 0

    def call(self, report, feat_idx, func, params=b""):
        """Send one request; return 'ok …', 'HID++ error …' or 'timeout'."""
        size = 20 if report == 0x11 else 64
        msg = bytes([report, 0xFF, feat_idx, (func << 4) | SW]) + params
        assert len(msg) <= size, (len(msg), size)
        msg = msg.ljust(size, b"\0")
        self.sent += 1
        if DRY:
            return "dry " + msg.hex(" ")
        os.write(self.fd, msg)
        deadline = time.monotonic() + 1.0
        while (left := deadline - time.monotonic()) > 0:
            r, _, _ = select.select([self.fd], [], [], left)
            if not r:
                break
            rep = os.read(self.fd, 64)
            if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx:
                return f"HID++ error {rep[5]:#04x}"
            if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
                return "ok " + rep[4:20].hex(" ")
        return "timeout"


def slot_of(dev, fid):
    if DRY:
        return {0x8080: 0x0C, 0x8070: 0x0D}[fid]
    r = dev.call(0x11, 0x00, 0, bytes([fid >> 8, fid & 0xFF]))
    if not r.startswith("ok") or r.split()[1] == "00":
        sys.exit(f"feature {fid:#06x} not found: {r}")
    return int(r.split()[1], 16)


def set_keys(dev, pk, colours):
    """colours: list of (key, (r, g, b)). Sends 14 per report; returns replies."""
    replies = []
    for i in range(0, len(colours), 14):
        body = bytearray([0x00, 0x01, 0x00, 0x0E])
        for key, (r, g, b) in colours[i:i + 14]:
            body += bytes([key, r, g, b])
        replies.append(dev.call(0x12, pk, 3, bytes(body)))
    return replies


def commit(dev, pk):
    return dev.call(0x11, pk, 5)


def effect(dev, cle, kind, block=()):
    return dev.call(0x11, cle, 3, bytes([ZONE, kind] + (list(block) + [0] * 10)[:10] + [0]))


def frame(dev, pk, colour, overrides=None):
    overrides = overrides or {}
    t = time.monotonic()
    replies = set_keys(dev, pk, [(k, overrides.get(k, colour)) for k in KEYS])
    replies.append(commit(dev, pk))
    return replies, time.monotonic() - t


def summary(replies):
    kinds = {}
    for r in replies:
        k = r.split(" 0x")[0] if r.startswith("HID++") else r.split()[0]
        kinds[k] = kinds.get(k, 0) + 1
    first_bad = next((r for r in replies if not r.startswith(("ok", "dry"))), None)
    return f"{kinds}" + (f", first problem: {first_bad}" if first_bad else "")


def listen(dev, secs):
    """Print HID++ reports only; count keyboard/media reports without contents."""
    counts = {}
    end = time.monotonic() + secs
    while (left := end - time.monotonic()) > 0:
        if DRY:
            time.sleep(min(left, 0.1))
            continue
        r, _, _ = select.select([dev.fd], [], [], left)
        if not r:
            break
        rep = os.read(dev.fd, 64)
        if rep[0] in (0x11, 0x12):
            log(f"  HID++ report: {rep[:20].hex(' ')}")
        else:
            counts[rep[0]] = counts.get(rep[0], 0) + 1
    return counts


def hold(secs):
    if not DRY:
        time.sleep(secs)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    path = args[0] if args else ("(dry run)" if DRY else find_dev())
    dev = Dev(path)
    pk, cle = slot_of(dev, 0x8080), slot_of(dev, 0x8070)
    log(f"device {path}, PerKeyLighting slot {pk:#04x}, ColorLedEffects slot {cle:#04x}")

    log("STEP 0  baseline: Static white (0x8070)")
    log(f"  {effect(dev, cle, STATIC, [255, 255, 255, 0])}")
    hold(3)

    log("STEP 1  Left Ctrl red over Static white (6 s)")
    log(f"  set   {set_keys(dev, pk, [(LEFT_CTRL, RED)])}")
    log(f"  commit {commit(dev, pk)}")
    hold(6)

    log("STEP 2  effect Off, then Esc green (6 s)")
    log(f"  off   {effect(dev, cle, OFF)}")
    hold(1)
    log(f"  set   {set_keys(dev, pk, [(ESC, GREEN)])}")
    log(f"  commit {commit(dev, pk)}")
    hold(5)

    log("STEP 3a full frame, all 104 keys blue (4 s)")
    replies, dt = frame(dev, pk, BLUE)
    log(f"  {len(replies)} requests in {dt * 1000:.0f} ms: {summary(replies)}")
    hold(4)

    log("STEP 3b fade: 40 full frames, blue 100% -> 20% -> 100%")
    if any(r == "timeout" for r in replies):
        log("  skipped: the full frame had timeouts, so 360 more requests would take minutes")
    else:
        fade(dev, pk)
    hold(2)

    log("STEP 4a start Wave (0x8070) over the blue frame (6 s)")
    log(f"  wave  {effect(dev, cle, WAVE, [0] * 6 + [30, 0, 1, 100])}")
    hold(6)

    log("STEP 4b Left Ctrl red while Wave runs (6 s)")
    log(f"  set   {set_keys(dev, pk, [(LEFT_CTRL, RED)])}")
    log(f"  commit {commit(dev, pk)}")
    hold(6)

    log("STEP 5a effect Off, then every key white 100% (8 s): compare Left Ctrl")
    log(f"  off   {effect(dev, cle, OFF)}")
    replies, _ = frame(dev, pk, WHITE)
    log(f"  {summary(replies)}")
    hold(8)

    for pct in (85, 70):
        v = round(255 * pct / 100)
        log(f"STEP 5{'b' if pct == 85 else 'c'} every key white, Left Ctrl at {pct}% (8 s)")
        replies, _ = frame(dev, pk, WHITE, {LEFT_CTRL: (v, v, v)})
        log(f"  {summary(replies)}")
        hold(8)

    log("STEP 6  keyboard amber: press the Game Mode key, wait, press it again (15 s)")
    replies, _ = frame(dev, pk, AMBER)
    log(f"  {summary(replies)}")
    counts = listen(dev, 15)
    log(f"  other reports (counted, not shown): {counts or 'none'}")

    log("STEP 7  end: effect Off (g512d restores the saved lighting next)")
    log(f"  off   {effect(dev, cle, OFF)}")
    log(f"done, {dev.sent} requests")


def fade(dev, pk):
    levels = [round(255 * (1 - 0.8 * min(i, 39 - i) / 19.5)) for i in range(40)]
    t, replies = time.monotonic(), []
    for lv in levels:
        replies += frame(dev, pk, (0, 0, lv))[0]
    dt = time.monotonic() - t
    log(f"  40 frames in {dt:.2f} s = {40 / dt if dt else 0:.1f} frames/s: {summary(replies)}")


if __name__ == "__main__":
    main()
