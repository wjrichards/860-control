#!/usr/bin/env python3
"""Spike 8: which button-table entry is which physical G502 HERO button, and
do keyboard entries written by us work?

Writes inactive user profile 5 only, and only if it equals the backup (the
spike 6 sector g512d also saved to ~/.config/g512-control). Keeps entries
0 and 1 (left and right click) and makes entries 2-10 type the letters c-k
(button entry 80 02 <modifiers> <HID usage>). Then makes profile 5 active,
so pressing each button types its letter.

Undo: g512d --restore-mouse-profiles ~/.config/g512-control/mouse-profiles-backup.txt
      busctl --user call io.github.g512control.Lighting1 /io/github/g512control/Lighting1 \\
          io.github.g512control.Lighting1 SetMouseActiveProfile y 4

Calls only: Root fn0; OnboardProfiles fn3 setCurrentProfile, fn4
getCurrentProfile, fn5 memoryRead, fn6/7/8 memory write.
Usage: python3 -I spike8_buttons.py /dev/hidrawN BACKUP
"""
import os, re, select, sys

SW = 0x0F
TARGET = 5
SIZE = 256
BUTTONS = 0x20          # button table: 4 bytes per button
LETTERS = "cdefghijk"   # entries 2..10
USAGE_C = 0x06          # HID usage of 'c'


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


def main():
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
    for i, letter in enumerate(LETTERS, start=2):
        edited[BUTTONS + 4 * i:BUTTONS + 4 * i + 4] = bytes([0x80, 0x02, 0x00, USAGE_C + ord(letter) - ord("c")])
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
    for i, letter in enumerate(LETTERS, start=2):
        print(f"  entry {i:2} types '{letter}'")
    os.close(fd)


if __name__ == "__main__":
    main()
