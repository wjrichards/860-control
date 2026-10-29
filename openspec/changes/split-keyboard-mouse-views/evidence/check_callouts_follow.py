"""Task 5.2: the callouts follow the active profile and action changes.
Sets the fake's MouseButtons and MouseActiveProfile over its Test interface,
the way a G9 press or another client would change them, and reads G9's
callout over AT-SPI after each step."""
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

sys.path.insert(0, "/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/keyboard-mouse-views-7d1c/target")
import a11y_views as v  # noqa: E402

NAME = "io.github.g512control.Lighting1Test"
bus = Gio.bus_get_sync(Gio.BusType.SESSION)

DEFAULT = [("mouse", 1, 0), ("mouse", 2, 0), ("mouse", 4, 0), ("mouse", 8, 0), ("mouse", 16, 0),
           ("special", 7, 0), ("special", 4, 0), ("special", 3, 0), ("special", 10, 0),
           ("special", 2, 0), ("special", 1, 0)]


def fake_set(prop, sig, value):
    bus.call_sync(NAME, "/io/github/g512control/Lighting1", "io.github.g512control.Test", "Set",
                  GLib.Variant("(sv)", (prop, GLib.Variant(sig, value))), None, 0, -1, None)


def buttons(g9_on_2):
    p2 = list(DEFAULT)
    p2[8] = g9_on_2
    return [(n, p2 if n == 2 else DEFAULT) for n in range(1, 6)]


def g9_callout():
    """G9's callout name, once the tree has settled (profile rows rebuild)."""
    for _ in range(10):
        time.sleep(0.5)
        try:
            names = {n.get_name() for n in v.walk(v.app(), []) if n.get_role_name() == "button" and (n.get_name() or "").startswith("6 G9")}
        except (GLib.GError, AttributeError):
            continue
        if names:
            return sorted(names)
    return []


fake_set("MouseButtons", "a(ya(sqy))", buttons(("key", 0x06, 0x01)))
print("profile 4, G9:", g9_callout())
fake_set("MouseActiveProfile", "y", 2)
print("switched to profile 2 (as G9 on the mouse), G9:", g9_callout())
fake_set("MouseButtons", "a(ya(sqy))", buttons(("media", 0xe9, 0)))
print("profile 2's G9 changed to Volume up, G9:", g9_callout())
fake_set("MouseActiveProfile", "y", 4)
print("back to profile 4, G9:", g9_callout())
