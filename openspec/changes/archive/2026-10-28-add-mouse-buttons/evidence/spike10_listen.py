#!/usr/bin/env python3
"""Spike 10: does the G502 HERO tell the host when G9 switches profiles?

Spike 2.4 found that getCurrentProfile (0x8100 fn4) keeps reporting the
profile the host set, even after G9 has switched the mouse to another one.
This listens to the mouse's HID++ interface and prints every report that
isn't a reply to our own two lookups, while the user presses G9.

Reads only: Root fn0 to find the slots of OnboardProfiles (0x8100) and
AdjustableDpi (0x2201), then listens. g512d keeps running; hidraw hands
every report to each reader.
Usage: python3 -I spike10_listen.py /dev/hidrawN SECONDS
"""
import os, select, sys, time

SW = 0x0D


def call(fd, idx, fn, params=b""):
    msg = bytes([0x11, 0xFF, idx, (fn << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 2.0)
        if not r:
            sys.exit("timeout")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == idx and rep[3] == ((fn << 4) | SW):
            return rep[4:20]


def main():
    path, seconds = sys.argv[1], float(sys.argv[2])
    fd = os.open(path, os.O_RDWR)
    slots = {call(fd, 0, 0, b"\x81\x00")[0]: "OnboardProfiles", call(fd, 0, 0, b"\x22\x01")[0]: "AdjustableDpi"}
    print("slots:", {f"{k:#04x}": v for k, v in slots.items()}, flush=True)
    end = time.monotonic() + seconds
    seen = 0
    while (left := end - time.monotonic()) > 0:
        r, _, _ = select.select([fd], [], [], left)
        if not r:
            break
        rep = os.read(fd, 64)
        if rep[0] not in (0x10, 0x11, 0x12):
            continue
        idx, fn_sw = rep[2], rep[3]
        # Replies carry a software ID in the low nibble; events carry 0.
        kind = "event" if fn_sw & 0x0F == 0 else f"reply (sw {fn_sw & 0x0F:#x})"
        name = slots.get(idx, f"slot {idx:#04x}")
        print(f"{time.strftime('%H:%M:%S')} {kind}: {name} fn/event {fn_sw >> 4}: {rep[:20].hex(' ')}", flush=True)
        seen += 1
    print(f"done: {seen} reports in {seconds:.0f} s")
    os.close(fd)


if __name__ == "__main__":
    main()
