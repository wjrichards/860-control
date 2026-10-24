#!/usr/bin/env python3
"""Spike 7: the first onboard-profile write on the G502 HERO, kept minimal
and reversible, plus switching the active profile.

Touches only user profile sector 5 (never the directory, sector 0, or the
ROM sectors 0x01xx), and only if:
  - profile 5 is not the active profile,
  - its CRC is valid,
  - its bytes equal the spike 6 backup exactly.
It changes one value (the first DPI step, 6400 -> 6000), reads the sector
back, then writes the original back and checks it matches the backup. Any
failure after the first write restores the original immediately.

Then it switches the active profile to 1 and back.

Calls only: Root fn0; OnboardProfiles (0x8100) fn2 getMode, fn3
setCurrentProfile, fn4 getCurrentProfile, fn5 memoryRead, fn6
memoryAddrWrite, fn7 memoryWrite, fn8 memoryWriteEnd; AdjustableDpi
(0x2201) fn2 getSensorDpi.

Usage: python3 -I spike7_profile_write.py /dev/hidrawN BACKUP(spike6-read.txt)
"""
import os, re, select, sys, time

SW = 0x0D
TARGET = 0x0005
SECTOR_SIZE = 256
DPI0 = 3  # profile format 2: report rate, default index, switched index, then DPI values (LE16)
OLD_DPI, NEW_DPI = 6400, 6000
t0 = time.monotonic()


def log(text):
    print(f"[{time.monotonic() - t0:5.1f}s] {text}", flush=True)


class Err(Exception):
    pass


def call(fd, feat_idx, func, params=b""):
    msg = bytes([0x11, 0xFF, feat_idx, (func << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 2.0)
        if not r:
            raise Err(f"timeout (slot {feat_idx:#04x} fn{func})")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx and rep[4] == ((func << 4) | SW):
            raise Err(f"HID++ error {rep[5]:#04x} (slot {feat_idx:#04x} fn{func})")
        if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
            return rep[4:20]


def slot(fd, feature):
    return call(fd, 0x00, 0, bytes([feature >> 8, feature & 0xFF]))[0]


def crc_ccitt(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else crc << 1
            crc &= 0xFFFF
    return crc


def crc_ok(data):
    return crc_ccitt(data[:-2]) == (data[-2] << 8 | data[-1])


def read_sector(fd, prof, sector):
    data = bytearray(SECTOR_SIZE)
    for off in list(range(0, SECTOR_SIZE - 15, 16)):
        data[off:off + 16] = call(fd, prof, 5, bytes([sector >> 8, sector & 0xFF, off >> 8, off & 0xFF]))
    return bytes(data)


def write_sector(fd, prof, sector, data):
    assert sector == TARGET and len(data) == SECTOR_SIZE and crc_ok(data)
    r = call(fd, prof, 6, bytes([sector >> 8, sector & 0xFF, 0, 0, SECTOR_SIZE >> 8, SECTOR_SIZE & 0xFF]))
    log(f"  memoryAddrWrite reply {r.hex(' ')}")
    for off in range(0, SECTOR_SIZE, 16):
        call(fd, prof, 7, data[off:off + 16])
    r = call(fd, prof, 8)
    log(f"  memoryWriteEnd reply {r.hex(' ')}")


def backup_sector(path, sector):
    text = open(path).read()
    m = re.search(rf"sector {sector:#06x}: crc ok\n  ([0-9a-f]+)", text)
    if not m:
        sys.exit(f"sector {sector:#06x} with a valid CRC not found in {path}")
    return bytes.fromhex(m.group(1))


def dpi(fd, dpi_slot):
    r = call(fd, dpi_slot, 2, b"\x00")
    return r[1] << 8 | r[2]


def main():
    path, backup_path = sys.argv[1], sys.argv[2]
    backup = backup_sector(backup_path, TARGET)
    fd = os.open(path, os.O_RDWR)
    prof, dpi_slot = slot(fd, 0x8100), slot(fd, 0x2201)
    log(f"device {path}: OnboardProfiles slot {prof:#04x}, AdjustableDpi slot {dpi_slot:#04x}")
    mode = call(fd, prof, 2)[0]
    current = call(fd, prof, 4)
    log(f"mode {mode} (1 = onboard), current profile {current[:2].hex(' ')}, DPI {dpi(fd, dpi_slot)}")
    if current[1] == TARGET:
        sys.exit("profile 5 is active; not writing to it")

    original = read_sector(fd, prof, TARGET)
    log(f"sector 5 read: crc {'ok' if crc_ok(original) else 'BAD'}, equals spike 6 backup: {original == backup}")
    if not crc_ok(original) or original != backup:
        sys.exit("sector 5 is not exactly the backed-up profile; stopping before any write")
    old = original[DPI0] | original[DPI0 + 1] << 8
    if old != OLD_DPI:
        sys.exit(f"first DPI step is {old}, expected {OLD_DPI}; stopping")

    edited = bytearray(original)
    edited[DPI0:DPI0 + 2] = NEW_DPI.to_bytes(2, "little")
    crc = crc_ccitt(edited[:-2])
    edited[-2:] = crc.to_bytes(2, "big")
    edited = bytes(edited)
    changed = [i for i in range(SECTOR_SIZE) if edited[i] != original[i]]
    log(f"edit: first DPI step {OLD_DPI} -> {NEW_DPI}; bytes changed at {changed}")

    wrote = False
    try:
        log("writing the edited sector 5")
        write_sector(fd, prof, TARGET, edited)
        wrote = True
        back = read_sector(fd, prof, TARGET)
        log(f"read back: equals the edit {back == edited}, crc {'ok' if crc_ok(back) else 'BAD'}, "
            f"first DPI step {back[DPI0] | back[DPI0 + 1] << 8}")
        if back != edited:
            raise Err("read-back differs from what was written")
    finally:
        if wrote or sys.exc_info()[0] is not None:
            log("restoring the original sector 5")
            try:
                write_sector(fd, prof, TARGET, original)
                again = read_sector(fd, prof, TARGET)
                log(f"restored: equals spike 6 backup {again == backup}, crc {'ok' if crc_ok(again) else 'BAD'}")
            except Err as e:
                log(f"RESTORE FAILED: {e} -- write the backup again before anything else")
                raise

    log("profile switching: to profile 1, then back")
    call(fd, prof, 3, bytes([0x00, 0x01]))
    time.sleep(1)
    log(f"  after setCurrentProfile 1: current {call(fd, prof, 4)[:2].hex(' ')}, DPI {dpi(fd, dpi_slot)}")
    time.sleep(3)
    call(fd, prof, 3, bytes(current[:2]))
    time.sleep(1)
    log(f"  after setCurrentProfile {current[:2].hex(' ')}: current {call(fd, prof, 4)[:2].hex(' ')}, "
        f"DPI {dpi(fd, dpi_slot)}, mode {call(fd, prof, 2)[0]}")
    os.close(fd)
    log("done")


if __name__ == "__main__":
    main()
