#!/usr/bin/env python3
"""Spike 9: do consumer (media) entries and an empty mouse-button mask work?

Writes inactive user profile 5 only, and only if it equals the backup (the
spike 6 sector g512d also saved to ~/.config/g512-control). Changes two
entries and keeps the rest:
  entry 2 (wheel click) -> 80 03 00 e9, consumer usage Volume Up (shows the
                           GNOME volume popup, so it's visible; play/pause
                           would need a player running);
  entry 8 (G9)          -> 80 01 00 00, the "disabled" candidate. On profile 5
                           G9 normally cycles profiles, so if it's really
                           disabled, pressing it leaves profile 5 active.
Then makes profile 5 active.

Undo: g512d --restore-mouse-profiles ~/.config/g512-control/mouse-profiles-backup.txt
      busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \\
          io.github.g512control.Lighting1 SetMouseActiveProfile y 4

Calls only: Root fn0; OnboardProfiles fn3 setCurrentProfile, fn4
getCurrentProfile, fn5 memoryRead, fn6/7/8 memory write.
Usage: python3 -I spike9_media_disabled.py /dev/hidrawN BACKUP
       python3 -I spike9_media_disabled.py --status /dev/hidrawN   (read only)
"""
import os, re, select, sys

SW = 0x0F
TARGET = 5
SIZE = 256
BUTTONS = 0x20          # button table: 4 bytes per button
CHANGES = {2: bytes([0x80, 0x03, 0x00, 0xE9]),   # wheel click: Volume Up
           8: bytes([0x80, 0x01, 0x00, 0x00])}   # G9: empty mouse-button mask


def call(fd, idx, fn, params=b""):
    msg = bytes([0x11, 0xFF, idx, (fn << 4) | SW]) + params
    os.write(fd, msg.ljust(20, b"\0"))
    while True:
        r, _, _ = select.select([fd], [], [], 2.0)
        if not r:
            sys.exit(f"timeout (fn{fn})")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == idx and rep[4] == ((fn << 4) | SW):
            sys.exit(f"HID++ error {rep[5]:#04x} (fn{fn})")
        if rep[0] in (0x11, 0x12) and rep[2] == idx and rep[3] == ((fn << 4) | SW):
            return rep[4:20]


def crc(data):
    c = 0xFFFF
    for b in data:
        c ^= b << 8
        for _ in range(8):
            c = ((c << 1) ^ 0x1021) & 0xFFFF if c & 0x8000 else (c << 1) & 0xFFFF
    return c


def read(fd, prof):
    return b"".join(call(fd, prof, 5, bytes([0, TARGET, 0, off])) for off in range(0, SIZE, 16))


def write(fd, prof, data):
    call(fd, prof, 6, bytes([0, TARGET, 0, 0, 1, 0]))
    for off in range(0, SIZE, 16):
        call(fd, prof, 7, data[off:off + 16])
    call(fd, prof, 8)


def table(data):
    return " ".join(data[BUTTONS + 4 * i:BUTTONS + 4 * i + 4].hex() for i in range(11))


def status(path):
    """Read only: the active profile and profile 5's button table."""
    fd = os.open(path, os.O_RDWR)
    prof = call(fd, 0, 0, b"\x81\x00")[0]
    print(f"active profile: {call(fd, prof, 4)[1]}")
    print("profile 5 buttons:", table(read(fd, prof)))
    os.close(fd)


def main():
    if sys.argv[1] == "--status":
        return status(sys.argv[2])
    path, backup_path = sys.argv[1], sys.argv[2]
    m = re.search(rf"sector {TARGET:#06x}: crc ok\n  ([0-9a-f]+)", open(backup_path).read())
    backup = bytes.fromhex(m.group(1))
    fd = os.open(path, os.O_RDWR)
    prof = call(fd, 0, 0, b"\x81\x00")[0]
    current = call(fd, prof, 4)[1]
    if current == TARGET:
        sys.exit("profile 5 is active; not writing to it")
    original = read(fd, prof)
    if original != backup:
        sys.exit("profile 5 differs from the backup; stopping before any write")
    print("profile 5 buttons before:", table(original))

    edited = bytearray(original)
    for i, entry in CHANGES.items():
        edited[BUTTONS + 4 * i:BUTTONS + 4 * i + 4] = entry
    edited[-2:] = crc(edited[:-2]).to_bytes(2, "big")
    edited = bytes(edited)
    print("profile 5 buttons after: ", table(edited))
    print("bytes changed:", [i for i in range(SIZE) if edited[i] != original[i]])
    write(fd, prof, edited)
    if read(fd, prof) != edited:
        print("READ-BACK DIFFERS: restoring")
        write(fd, prof, original)
        sys.exit(1)
    print("written and read back")
    call(fd, prof, 3, bytes([0, TARGET]))
    print(f"active profile now {call(fd, prof, 4)[1]} (was {current})")
    print("  wheel click should raise the volume; G9 should do nothing")
    os.close(fd)


if __name__ == "__main__":
    main()
