#!/usr/bin/env python3
"""Spike 10 (task 1.1): read-only dump of the G502 HERO's onboard memory,
sectors 0 to the sector count, for the macro work.

Only read functions are called:
  Root (0x0000) fn0 getFeature
  OnboardProfiles (0x8100) fn0 getInfo, fn5 memoryRead
Nothing is set and no memory is written. Software ID 0x0A, so replies to
g512d's own requests (it may be running) are ignored.

Usage: python3 -I spike10_read.py [/dev/hidrawN] | tee spike10-read.txt
"""
import os, select, sys

SW = 0x0A
READ_ONLY = {(0x00, 0)}  # (feature slot, function) pairs allowed; 0x8100's slot is added once resolved


def find_dev():
    for n in sorted(os.listdir("/sys/class/hidraw")):
        ue = open(f"/sys/class/hidraw/{n}/device/uevent").read()
        rd = open(f"/sys/class/hidraw/{n}/device/report_descriptor", "rb").read()
        if "046D:0000C08B" in ue.upper() and b"\x85\x11" in rd:
            return f"/dev/{n}"
    sys.exit("G502 HERO HID++ interface not found")


def call(fd, slot, func, params=b""):
    if (slot, func) not in READ_ONLY:
        sys.exit(f"refused: slot {slot:#04x} fn{func} is not a read")
    msg = bytes([0x11, 0xFF, slot, (func << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 1.0)
        if not r:
            raise RuntimeError(f"timeout (slot {slot:#04x} fn{func})")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == slot and rep[4] == ((func << 4) | SW):
            raise RuntimeError(f"HID++ error {rep[5]:#04x}")
        if rep[0] in (0x11, 0x12) and rep[2] == slot and rep[3] == ((func << 4) | SW):
            return rep[4:20]


def crc_ccitt(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else crc << 1
            crc &= 0xFFFF
    return crc


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else find_dev()
    fd = os.open(path, os.O_RDWR)
    print(f"device: {path}")
    slot = call(fd, 0x00, 0, bytes([0x81, 0x00]))[0]
    print(f"OnboardProfiles (0x8100) slot: {slot:#04x}")
    READ_ONLY.update({(slot, 0), (slot, 5)})
    info = call(fd, slot, 0)
    sectors, size = info[6], info[7] << 8 | info[8]
    print(f"getInfo: {info.hex(' ')}")
    print(f"  profile_format={info[1]} macro_format={info[2]} profiles={info[3]} buttons={info[5]} sectors={sectors} sector_size={size}")
    for sector in range(sectors):
        data = bytearray(size)
        for off in range(0, size, 16):
            data[off:off + 16] = call(fd, slot, 5, bytes([sector >> 8, sector & 0xFF, off >> 8, off & 0xFF]))
        stored, computed = data[-2] << 8 | data[-1], crc_ccitt(data[:-2])
        if all(b == 0xFF for b in data):
            state = "blank (all ff)"
        elif stored == computed:
            state = "crc ok"
        else:
            state = f"crc not valid (stored {stored:#06x}, computed {computed:#06x})"
        print(f"sector {sector:#06x}: {state}; last two bytes {data[-2]:02x} {data[-1]:02x}")
        print(f"  {data.hex()}")
    os.close(fd)


if __name__ == "__main__":
    main()
