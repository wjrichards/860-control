"""A stand-in for g512d, for testing the tile and the app.

It owns io.github.g512control.Lighting1 (on a private bus) with the members the tile and the app use,
logs every call to stdout ("<seconds> call <Method> <args>"), and takes property changes from the test through
a second interface, io.github.g512control.Test:

    Set(s name, v value)   change a property and emit PropertiesChanged
    Quit()

Options:
    --yes-this-is-a-private-bus   required to own the real name
    --name NAME                   own NAME instead; safe on the real session bus (the app
                                  follows it with G512_CONTROL_BUS_NAME=NAME)
    --keyboard-missing            report the keyboard as disconnected
    --no-mouse                    an older service: no mouse members at all

Run with python3 -I. Never own the real name on the real session bus.
"""
import sys
import time

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

NAME = "io.github.g512control.Lighting1"
PATH = "/io/github/g512control/Lighting1"

XML = """
<node>
  <interface name="io.github.g512control.Lighting1">
    <property name="DeviceState" type="s" access="read"/>
    <property name="DeviceName" type="s" access="read"/>
    <property name="Firmware" type="s" access="read"/>
    <property name="Effects" type="a(sbbyy)" access="read"/>
    <property name="State" type="(s(yyy)yy)" access="read"/>
    <property name="Pending" type="b" access="read"/>
    <property name="GameSwitch" type="b" access="read"/>
    <property name="GameActive" type="b" access="read"/>
    <property name="LightsOn" type="b" access="read"/>
    <property name="CustomKeys" type="a(yyyy)" access="read"/>
    <property name="GamingKeys" type="a(yyyy)" access="read"/>
    MOUSE_PROPERTIES
    <method name="SetState">
      <arg type="s" direction="in"/><arg type="(yyy)" direction="in"/>
      <arg type="y" direction="in"/><arg type="y" direction="in"/>
      <arg type="b" direction="out"/>
    </method>
    <method name="SetGameSwitch"><arg type="b" direction="in"/></method>
    <method name="SetLightsOn"><arg type="b" direction="in"/></method>
  </interface>
  <interface name="io.github.g512control.Test">
    <method name="Set"><arg type="s" direction="in"/><arg type="v" direction="in"/></method>
    <method name="Quit"/>
  </interface>
</node>
"""

SIGNATURES = {
    "DeviceState": "s", "DeviceName": "s", "Firmware": "s", "Effects": "a(sbbyy)",
    "State": "(s(yyy)yy)", "Pending": "b", "GameSwitch": "b", "GameActive": "b", "LightsOn": "b",
    "CustomKeys": "a(yyyy)", "GamingKeys": "a(yyyy)",
}
MOUSE_SIGNATURES = {
    "MouseState": "s", "MouseName": "s", "MouseEffects": "a(sbbyy)", "MouseMatch": "b",
    "MouseLighting": "(s(yyy)yy)", "MouseProfiles": "a(yyyaq)", "MouseActiveProfile": "y",
    "MouseDpiRange": "(qqq)", "MouseButtons": "a(ya(sqy))",
}
PROFILE_BUTTONS = [("mouse", 1, 0), ("mouse", 2, 0), ("mouse", 4, 0), ("mouse", 8, 0), ("mouse", 16, 0),
                   ("special", 7, 0), ("special", 4, 0), ("special", 3, 0), ("special", 10, 0),
                   ("special", 2, 0), ("special", 1, 0)]

props = {
    "DeviceState": "connected",
    "DeviceName": "G512 RGB MECHANICAL GAMING KEYBOARD",
    "Firmware": "U1 19.01 build 0014",
    "Effects": [
        ("Off", False, False, 0, 0), ("Static", True, False, 0, 0), ("Breathe", True, True, 1, 100),
        ("Cycle", False, True, 1, 100), ("Wave", False, True, 1, 100), ("Ripple", True, True, 1, 100),
        ("Custom", False, False, 0, 0),
    ],
    "State": ("Ripple", (0x3D, 0x38, 0x46), 52, 100),
    "Pending": False,
    "GameSwitch": False,
    "GameActive": False,
    "LightsOn": True,
    "CustomKeys": [(0x1a, 0x26, 0xA2, 0x69), (0x04, 0x26, 0xA2, 0x69), (0x16, 0x26, 0xA2, 0x69), (0x07, 0x26, 0xA2, 0x69)],
    "GamingKeys": [],
}
mouse_props = {
    "MouseState": "connected",
    "MouseName": "G502 HERO Gaming Mouse",
    "MouseEffects": [("Off", False, False, 0, 0), ("Static", True, False, 0, 0),
                     ("Breathe", True, True, 1, 100), ("Cycle", False, True, 1, 100)],
    "MouseMatch": True,
    "MouseLighting": ("Static", (0x26, 0xA2, 0x69), 50, 100),
    "MouseProfiles": [(n, 1, 0, [3200]) for n in range(1, 6)],
    "MouseActiveProfile": 4,
    "MouseDpiRange": (100, 25600, 50),
    "MouseButtons": [(n, PROFILE_BUTTONS) for n in range(1, 6)],
}

loop = GLib.MainLoop()
conn = None


def log(text):
    print(text, flush=True)


def changed(*names):
    body = {n: GLib.Variant(SIGNATURES[n], props[n]) for n in names}
    conn.emit_signal(None, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged",
                     GLib.Variant("(sa{sv}as)", ("io.github.g512control.Lighting1", body, [])))


def on_call(_conn, _sender, _path, iface, method, params, invocation):
    args = params.unpack()
    if iface == "io.github.g512control.Test":
        log(f"test {method} {args!r}")
        if method == "Set":
            name, value = args
            props[name] = value
            changed(name)
        else:
            loop.quit()
        invocation.return_value(None)
        return
    log(f"{time.monotonic():.3f} call {method} {args!r}")
    if method == "SetState":
        effect, color, speed, brightness = args
        if effect not in [e[0] for e in props["Effects"]] or brightness > 100 or not 1 <= speed <= 100:
            invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", f"invalid state {args!r}")
            return
        props["State"] = (effect, tuple(color), speed, brightness)
        props["LightsOn"] = True
        props["GameActive"] = False
        changed("State", "LightsOn", "GameActive")
        invocation.return_value(GLib.Variant("(b)", (False,)))
    elif method == "SetGameSwitch":
        props["GameSwitch"] = args[0]
        changed("GameSwitch")
        invocation.return_value(None)
    elif method == "SetLightsOn":
        props["LightsOn"] = args[0]
        changed("LightsOn", "GameActive")
        invocation.return_value(None)


def on_get(_conn, _sender, _path, _iface, name):
    return GLib.Variant(SIGNATURES[name], props[name])


def on_bus(c, _name):
    global conn
    conn = c
    mouse = "" if "--no-mouse" in sys.argv else "\n".join(
        f'<property name="{n}" type="{t}" access="read"/>' for n, t in MOUSE_SIGNATURES.items())
    for info in Gio.DBusNodeInfo.new_for_xml(XML.replace("MOUSE_PROPERTIES", mouse)).interfaces:
        c.register_object(PATH, info, on_call, on_get, None)


def main():
    name = sys.argv[sys.argv.index("--name") + 1] if "--name" in sys.argv else NAME
    if name == NAME and "--yes-this-is-a-private-bus" not in sys.argv:
        sys.exit("refusing to own the real name without --yes-this-is-a-private-bus")
    if "--keyboard-missing" in sys.argv:
        props["DeviceState"] = "disconnected"
    if "--no-mouse" not in sys.argv:
        SIGNATURES.update(MOUSE_SIGNATURES)
        props.update(mouse_props)
    Gio.bus_own_name(Gio.BusType.SESSION, name, Gio.BusNameOwnerFlags.NONE, on_bus,
                     lambda *_: log("ready"), lambda *_: (log("name lost"), loop.quit()))
    loop.run()


if __name__ == "__main__":
    main()
