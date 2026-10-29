"""A stand-in for g512d, for testing the tile and the app.

It owns io.github.g512control.Lighting1 (on a private bus) with the members the tile and the app use,
logs every call to stdout ("<seconds> call <Method> <args>"), and takes property changes from the test through
a second interface, io.github.g512control.Test:

    Set(s name, v value)   change a property and emit PropertiesChanged
    FailNext(s problem)    make the next SaveMouseMacro fail with too-long, full or write
    Quit()

Options:
    --yes-this-is-a-private-bus   required to own the real name
    --name NAME                   own NAME instead; safe on the real session bus (the app
                                  follows it with G512_CONTROL_BUS_NAME=NAME)
    --keyboard-missing            report the keyboard as disconnected
    --no-mouse                    an older service: no mouse members at all
    --no-macros                   an older service: mouse members, but no macros

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
    MOUSE_METHODS
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
    <method name="FailNext"><arg type="s" direction="in"/></method>
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
MACRO_SIGNATURES = {"MouseMacros": "a(yssa(sqy))", "MouseMacroSectors": "(yy)"}
MOUSE_METHODS = """
    <method name="SetMouseButton">
      <arg type="y" direction="in"/><arg type="y" direction="in"/><arg type="s" direction="in"/>
      <arg type="q" direction="in"/><arg type="y" direction="in"/>
    </method>"""
MACRO_METHODS = """
    <method name="SaveMouseMacro">
      <arg type="y" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="in"/>
      <arg type="a(sqy)" direction="in"/><arg type="y" direction="out"/>
    </method>
    <method name="DeleteMouseMacro"><arg type="y" direction="in"/></method>"""
MACRO_SECTORS = range(6, 16)
SECTOR_SIZE = 256


def tap(usage, modifiers=0):
    return [("key-down", usage, modifiers), ("key-up", usage, modifiers)]


PAUSE_50 = [("pause", 50, 0)]
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
# The four macros on the design boards (05 Screens): G9 plays Copy line in profile 4,
# DPI Up plays Reload in profiles 1 and 2, and the macro in sector 9 has no name.
macro_props = {
    "MouseMacros": [
        (6, "Copy line", "once", tap(0x4a) + PAUSE_50 + tap(0x4d, 2) + PAUSE_50 + tap(0x06, 1)),
        (7, "Reload", "held", tap(0x15) + PAUSE_50 + tap(0x15) + PAUSE_50 + tap(0x15) + tap(0x2c)),
        (8, "Sprint toggle", "toggle", [("key-down", 0xe1, 0), ("key-up", 0xe1, 0)]),
        (9, "", "once", tap(0x04) + tap(0x05) + tap(0x06) + tap(0x07)),
    ],
    "MouseMacroSectors": (4, 10),
}
fail_next = None

loop = GLib.MainLoop()
conn = None


def log(text):
    print(text, flush=True)


def changed(*names):
    body = {n: GLib.Variant(SIGNATURES[n], props[n]) for n in names}
    conn.emit_signal(None, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged",
                     GLib.Variant("(sa{sv}as)", ("io.github.g512control.Lighting1", body, [])))


def macro_size(mode, steps):
    """Bytes the macro takes on the mouse, with its end command."""
    return (mode != "once") + sum(1 if kind == "wait-release" else 3 for kind, _, _ in steps) + 1


def with_button(profile, button, action):
    props["MouseButtons"] = [(n, [action if (n, i) == (profile, button) else a for i, a in enumerate(actions)])
                             for n, actions in props["MouseButtons"]]


def save_macro(args, invocation):
    global fail_next
    sector, name, mode, steps = args
    used = {m[0] for m in props["MouseMacros"]}
    problem, fail_next = fail_next, None
    error = "io.github.g512control.Error."
    if mode not in ("once", "held", "toggle"):
        return invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", f"unknown play mode {mode!r}")
    if problem == "too-long" or macro_size(mode, steps) > SECTOR_SIZE - 2:
        return invocation.return_dbus_error(error + "MacroTooLong", "the macro doesn't fit in one sector")
    if sector == 0:
        free = [s for s in MACRO_SECTORS if s not in used]
        if problem == "full" or not free:
            return invocation.return_dbus_error(error + "MacroMemoryFull", "every macro sector on the mouse is in use")
        sector = free[0]
    elif sector not in used:
        return invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", f"sector {sector} holds no macro")
    if problem == "write":
        return invocation.return_dbus_error(error + "WriteFailed", "read back differently; the mouse is unchanged")
    macros = [m for m in props["MouseMacros"] if m[0] != sector] + [(sector, name.strip(), mode, steps)]
    props["MouseMacros"] = sorted(macros)
    props["MouseMacroSectors"] = (len(macros), len(MACRO_SECTORS))
    changed("MouseMacros", "MouseMacroSectors")
    invocation.return_value(GLib.Variant("(y)", (sector,)))


def on_call(_conn, _sender, _path, iface, method, params, invocation):
    global fail_next
    args = params.unpack()
    if iface == "io.github.g512control.Test":
        log(f"test {method} {args!r}")
        if method == "Set":
            name, value = args
            props[name] = value
            changed(name)
        elif method == "FailNext":
            fail_next = args[0]
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
    elif method == "SetMouseButton":
        profile, button, kind, value, modifiers = args
        if button == 0 or (kind == "macro" and value not in {m[0] for m in props.get("MouseMacros", [])}):
            invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", f"refused {args!r}")
            return
        with_button(profile, button, (kind, value, modifiers))
        changed("MouseButtons")
        invocation.return_value(None)
    elif method == "SaveMouseMacro":
        save_macro(args, invocation)
    elif method == "DeleteMouseMacro":
        sector = args[0]
        if sector not in {m[0] for m in props["MouseMacros"]}:
            invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", f"sector {sector} holds no macro")
            return
        for n, actions in props["MouseButtons"]:
            for i, action in enumerate(actions):
                if action[:2] == ("macro", sector):
                    with_button(n, i, ("disabled", 0, 0))
        props["MouseMacros"] = [m for m in props["MouseMacros"] if m[0] != sector]
        props["MouseMacroSectors"] = (len(props["MouseMacros"]), len(MACRO_SECTORS))
        changed("MouseButtons", "MouseMacros", "MouseMacroSectors")
        invocation.return_value(None)


def on_get(_conn, _sender, _path, _iface, name):
    return GLib.Variant(SIGNATURES[name], props[name])


def on_bus(c, _name):
    global conn
    conn = c
    mouse = "" if "--no-mouse" in sys.argv else "\n".join(
        f'<property name="{n}" type="{t}" access="read"/>' for n, t in MOUSE_SIGNATURES.items())
    methods = "" if "--no-mouse" in sys.argv else MOUSE_METHODS
    if "--no-mouse" not in sys.argv and "--no-macros" not in sys.argv:
        mouse += "\n".join(f'<property name="{n}" type="{t}" access="read"/>' for n, t in MACRO_SIGNATURES.items())
        methods += MACRO_METHODS
    xml = XML.replace("MOUSE_PROPERTIES", mouse).replace("MOUSE_METHODS", methods)
    for info in Gio.DBusNodeInfo.new_for_xml(xml).interfaces:
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
        # G9 (button 8) plays Copy line in profile 4; DPI Up (button 7) plays Reload in profiles 1 and 2.
        if "--no-macros" not in sys.argv:
            SIGNATURES.update(MACRO_SIGNATURES)
            props.update(macro_props)
            with_button(4, 8, ("macro", 6, 0))
            with_button(1, 7, ("macro", 7, 0))
            with_button(2, 7, ("macro", 7, 0))
    Gio.bus_own_name(Gio.BusType.SESSION, name, Gio.BusNameOwnerFlags.NONE, on_bus,
                     lambda *_: log("ready"), lambda *_: (log("name lost"), loop.quit()))
    loop.run()


if __name__ == "__main__":
    main()
