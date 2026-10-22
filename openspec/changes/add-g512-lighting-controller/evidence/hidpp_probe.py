#!/usr/bin/env python3
"""Read-only HID++ 2.0 feature enumeration for the Logitech G512 (046d:c33c).

Only sends Root.getFeature / FeatureSet.getCount / FeatureSet.getFeatureID /
DeviceName / FW queries. Does not change lighting or any device state.
Usage: sudo python3 -I hidpp_probe.py [/dev/hidrawN]
"""
import os, select, sys

NAMES = {
    0x0000: "Root", 0x0001: "FeatureSet", 0x0002: "FeatureInfo", 0x0003: "DeviceFwVersion",
    0x0005: "DeviceName", 0x0007: "DeviceFriendlyName", 0x0020: "ConfigChange",
    0x00C2: "DfuControlSigned", 0x00D0: "Dfu", 0x1802: "DeviceReset", 0x1803: "GpioAccess",
    0x1806: "ConfigurableDeviceProperties", 0x1814: "ChangeHost", 0x1830: "PowerModes",
    0x1890: "RfTest", 0x18A1: "LedTest", 0x1B04: "ReprogControlsV4", 0x1E00: "EnableHiddenFeatures",
    0x1E22: "SpiDirectAccess", 0x1EB0: "TdeAccess", 0x4522: "DisableKeysByUsage",
    0x4540: "KeyboardInternationalLayouts", 0x8010: "GKey", 0x8020: "MKeys", 0x8030: "MRKey",
    0x8040: "BrightnessControl", 0x8060: "AdjustableReportRate", 0x8070: "ColorLedEffects",
    0x8071: "RgbEffects", 0x8080: "PerKeyLighting", 0x8081: "PerKeyLightingV2",
    0x8100: "OnboardProfiles", 0x8110: "MouseButtonSpy",
}
SW = 0x0A  # arbitrary software id


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
            raise TimeoutError(f"no reply to feat {feat_idx:#x} func {func}")
        rep = os.read(fd, 64)
        if rep[0] in (0x11, 0x12) and rep[2] == 0xFF and rep[3] == feat_idx:
            raise RuntimeError(f"HID++ error {rep[5]:#x} for feat {feat_idx:#x} func {func}")
        if rep[0] in (0x11, 0x12) and rep[2] == feat_idx and rep[3] == ((func << 4) | SW):
            return rep[4:]


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else find_dev()
    fd = os.open(path, os.O_RDWR)
    print(f"device: {path}")
    proto = call(fd, 0x00, 1, b"\0\0\x5a")
    print(f"HID++ protocol {proto[0]}.{proto[1]}")

    fs_idx = call(fd, 0x00, 0, b"\x00\x01")[0]
    count = call(fd, fs_idx, 0)[0]
    feats = {}
    print(f"\n{'idx':>4}  {'id':>6}  ver  flags  name")
    for i in range(count + 1):
        r = call(fd, fs_idx, 1, bytes([i])) if i else b"\x00\x00\x00\x00"
        fid, flags, ver = (r[0] << 8) | r[1], r[2], r[3]
        feats[fid] = i
        print(f"{i:#04x}  {fid:#06x}  {ver:>3}  {flags:#04x}   {NAMES.get(fid, '?')}")

    if 0x0005 in feats:
        n = call(fd, feats[0x0005], 0)[0]
        name = b""
        while len(name) < n:
            name += call(fd, feats[0x0005], 1, bytes([len(name)]))[:16]
        print(f"\nname: {name[:n].decode(errors='replace')}")
    if 0x0003 in feats:
        nent = call(fd, feats[0x0003], 0)[0]
        for e in range(nent):
            r = call(fd, feats[0x0003], 1, bytes([e]))
            print(f"fw[{e}]: type={r[0]} {r[1:4].decode(errors='replace')}"
                  f"{r[4]:02x}.{r[5]:02x} build {r[6]:02x}{r[7]:02x}")
    for fid in (0x8070, 0x8071):
        if fid in feats:
            r = call(fd, feats[fid], 0)  # getInfo
            print(f"{NAMES[fid]} getInfo: {r[:16].hex(' ')}")
    for fid in (0x8080, 0x8081):
        if fid in feats:
            r = call(fd, feats[fid], 0, b"\0")  # getInfo(type 0)
            print(f"{NAMES[fid]} getInfo: {r[:16].hex(' ')}")
    os.close(fd)


if __name__ == "__main__":
    main()
