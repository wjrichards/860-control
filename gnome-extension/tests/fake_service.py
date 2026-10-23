"""A stand-in for g512d on a private session bus, for testing the tile.

It owns io.github.g512control.Lighting1 with the members the tile uses,
logs every call to stdout ("<seconds> call <Method> <args>"), and takes property changes from the test through
a second interface, io.github.g512control.Test:

    Set(s name, v value)   change a property and emit PropertiesChanged
    Quit()

Run with python3 -I. Never use this on the real session bus.
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
}

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
    for info in Gio.DBusNodeInfo.new_for_xml(XML).interfaces:
        c.register_object(PATH, info, on_call, on_get, None)


def main():
    if "--yes-this-is-a-private-bus" not in sys.argv:
        sys.exit("refusing to start without --yes-this-is-a-private-bus")
    Gio.bus_own_name(Gio.BusType.SESSION, NAME, Gio.BusNameOwnerFlags.NONE, on_bus,
                     lambda *_: log("ready"), lambda *_: (log("name lost"), loop.quit()))
    loop.run()


if __name__ == "__main__":
    main()
