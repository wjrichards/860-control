#!/usr/bin/env python3
"""Spike 10 (tasks 1.2-1.4): one supervised macro write on the G502 HERO.

  spike10_write.py write             sector 6 = "a, b" played once; profile 5 G9 (entry 8) -> 00 06 00 00
  spike10_write.py macro MODE [MS]   rewrite sector 6 only: MODE is none, 01, 02 or 03 before the steps,
                                     MS adds a pause of that many milliseconds between a and b
  spike10_write.py restore           sectors 5 and 6 back to the bytes in spike10-read.txt

Only sectors 5 and 6 can be written; the backup is spike10-read.txt (task 1.1).
Features used: Root (0x0000) getFeature; OnboardProfiles (0x8100) getInfo,
getCurrentProfile, memoryRead, memoryAddrWrite, memoryWrite, memoryWriteEnd.
Every write is read back and compared.
"""
import os, re, select, sys

SW = 0x0A
WRITABLE = {5, 6}
HERE = os.path.dirname(os.path.abspath(__file__))


def find_dev():
    for n in sorted(os.listdir("/sys/class/hidraw")):
        ue = open(f"/sys/class/hidraw/{n}/device/uevent").read()
        rd = open(f"/sys/class/hidraw/{n}/device/report_descriptor", "rb").read()
        if "046D:0000C08B" in ue.upper() and b"\x85\x11" in rd:
            return f"/dev/{n}"
    sys.exit("G502 HERO HID++ interface not found")


class Dev:
    def __init__(self):
        self.fd = os.open(find_dev(), os.O_RDWR)
        self.slot = self.call(0x00, 0, bytes([0x81, 0x00]))[0]
        info = self.call(self.slot, 0)
        self.size = info[7] << 8 | info[8]

    def call(self, slot, func, params=b""):
        os.write(self.fd, (bytes([0x11, 0xFF, slot, (func << 4) | SW]) + params).ljust(20, b"\0"))
        while True:
            r, _, _ = select.select([self.fd], [], [], 1.0)
            if not r:
                raise RuntimeError(f"timeout (slot {slot:#04x} fn{func})")
            rep = os.read(self.fd, 64)
            if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == slot and rep[4] == ((func << 4) | SW):
                raise RuntimeError(f"HID++ error {rep[5]:#04x} (fn{func})")
            if rep[0] in (0x11, 0x12) and rep[2] == slot and rep[3] == ((func << 4) | SW):
                return rep[4:20]

    def current_profile(self):
        return self.call(self.slot, 4)[1]

    def read(self, sector):
        out = bytearray()
        for off in range(0, self.size, 16):
            out += self.call(self.slot, 5, bytes([0, sector, off >> 8, off & 0xFF]))
        return bytes(out)

    def write(self, sector, data):
        if sector not in WRITABLE:
            sys.exit(f"refused: sector {sector} is not one this spike may write")
        if len(data) != self.size or crc(data[:-2]) != (data[-2] << 8 | data[-1]):
            sys.exit(f"refused: sector {sector} data has the wrong size or CRC")
        self.call(self.slot, 6, bytes([0, sector, 0, 0, self.size >> 8, self.size & 0xFF]))
        for off in range(0, self.size, 16):
            self.call(self.slot, 7, data[off:off + 16])
        self.call(self.slot, 8)
        back = self.read(sector)
        print(f"sector {sector}: written, read-back {'matches' if back == data else 'DIFFERS'}")
        if back != data:
            print(f"  read back: {back.hex()}")
            sys.exit(1)


def crc(data):
    c = 0xFFFF
    for byte in data:
        c ^= byte << 8
        for _ in range(8):
            c = ((c << 1) ^ 0x1021) if c & 0x8000 else c << 1
            c &= 0xFFFF
    return c


def with_crc(body, size):
    body = bytes(body).ljust(size - 2, b"\xff")
    c = crc(body)
    return body + bytes([c >> 8, c & 0xFF])


def backup():
    t = open(os.path.join(HERE, "spike10-read.txt")).read().splitlines()
    out = {}
    for i, line in enumerate(t):
        m = re.match(r"sector (0x[0-9a-f]+):", line)
        if m:
            out[int(m.group(1), 16)] = bytes.fromhex(t[i + 1].strip())
    return out


def macro(mode, pause_ms):
    a_down, a_up, b_down, b_up = [0x43, 0, 0x04], [0x44, 0, 0x04], [0x43, 0, 0x05], [0x44, 0, 0x05]
    # 01 (wait for release) goes between a and b, so a plays on press and b on release;
    # 02 and 03 (repeat modes) go first.
    body = [] if mode in ("none", "01") else [int(mode, 16)]
    body += a_down + a_up
    if mode == "01":
        body += [0x01]
    if pause_ms:
        body += [0x40, pause_ms >> 8, pause_ms & 0xFF]
    body += b_down + b_up + [0xFF]
    return body


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    dev, bak = Dev(), backup()
    print(f"active profile: {dev.current_profile()}")
    if cmd == "write":
        if dev.current_profile() != 5:
            sys.exit("refused: switch the mouse to profile 5 first")
        for s in (5, 6):
            if dev.read(s) != bak[s]:
                sys.exit(f"refused: sector {s} no longer matches the backup")
        body = macro("none", 0)
        print(f"macro: {bytes(body).hex(' ')}")
        dev.write(6, with_crc(body, dev.size))
        prof = bytearray(bak[5])
        print(f"profile 5 entry 8 was {prof[0x40:0x44].hex(' ')}")
        prof[0x40:0x44] = bytes([0x00, 0x06, 0x00, 0x00])
        dev.write(5, with_crc(prof[:-2], dev.size))
    elif cmd == "macro":
        mode, ms = sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 0
        body = macro(mode, ms)
        print(f"macro: {bytes(body).hex(' ')}")
        dev.write(6, with_crc(body, dev.size))
    elif cmd == "restore":
        for s in (6, 5):
            dev.write(s, bak[s]) if crc(bak[s][:-2]) == (bak[s][-2] << 8 | bak[s][-1]) else restore_raw(dev, s, bak[s])
    else:
        sys.exit(__doc__)


def restore_raw(dev, sector, data):
    # Sector 6's original bytes (all zero) have no valid CRC, so they bypass the CRC check here only.
    if sector not in WRITABLE:
        sys.exit("refused")
    dev.call(dev.slot, 6, bytes([0, sector, 0, 0, dev.size >> 8, dev.size & 0xFF]))
    for off in range(0, dev.size, 16):
        dev.call(dev.slot, 7, data[off:off + 16])
    dev.call(dev.slot, 8)
    back = dev.read(sector)
    print(f"sector {sector}: restored raw, read-back {'matches' if back == data else 'DIFFERS'}")


if __name__ == "__main__":
    main()
