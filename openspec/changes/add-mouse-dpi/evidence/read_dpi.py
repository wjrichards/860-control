#!/usr/bin/env python3
"""Read the G502 HERO's current DPI, active profile and DPI index (reads only:
Root fn0, AdjustableDpi fn2, OnboardProfiles fn4 and fn11).
Usage: python3 -I read_dpi.py /dev/hidrawN"""
import os, select, sys

SW = 0x0E


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


fd = os.open(sys.argv[1], os.O_RDWR)
dpi_slot = call(fd, 0, 0, b"\x22\x01")[0]
prof_slot = call(fd, 0, 0, b"\x81\x00")[0]
d = call(fd, dpi_slot, 2, b"\x00")
print(f"current DPI {d[1] << 8 | d[2]}, active profile {call(fd, prof_slot, 4)[1]}, DPI index {call(fd, prof_slot, 11)[0]}")
