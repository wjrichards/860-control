"""Probe the app's views over AT-SPI.

  a11y_views.py show            switcher buttons, visible groups and status titles
  a11y_views.py switch NAME     press the view switcher button NAME (Keyboard / Mouse)
"""
import sys

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402


def app():
    d = Atspi.get_desktop(0)
    for i in range(d.get_child_count()):
        a = d.get_child_at_index(i)
        if a and (a.get_name() or "") == "g512-control":
            return a
    sys.exit("g512-control is not on the accessibility bus")


def walk(n, out, depth=0):
    if depth > 60:
        return out
    try:
        out.append(n)
        for i in range(n.get_child_count()):
            walk(n.get_child_at_index(i), out, depth + 1)
    except Exception:
        pass
    return out


def showing(n):
    s = n.get_state_set()
    return s.contains(Atspi.StateType.SHOWING) and s.contains(Atspi.StateType.VISIBLE)


def main():
    nodes = walk(app(), [])
    if sys.argv[1] == "show":
        for n in nodes:
            role, name = n.get_role_name(), n.get_name() or ""
            if not showing(n) or not name:
                continue
            if role in ("toggle button", "page tab", "radio button") and name in ("Keyboard", "Mouse"):
                checked = n.get_state_set().contains(Atspi.StateType.CHECKED)
                print(f"switcher: {name}{' (selected)' if checked else ''}")
            elif role == "grouping":
                print(f"group: {name}")
            elif role == "heading" or (role == "label" and name.endswith(("Connected", "Running", "Mouse", "Keyboard", "Supported"))):
                print(f"{role}: {name}")
    elif sys.argv[1] == "switch":
        for n in nodes:
            if (n.get_name() or "") == sys.argv[2] and n.get_role_name() in ("toggle button", "page tab", "radio button") and showing(n):
                a = n.get_action_iface()
                if a and a.get_n_actions():
                    a.do_action(0)
                    print(f"pressed {sys.argv[2]}")
                    return
        sys.exit(f"no switcher button {sys.argv[2]!r}")


if __name__ == "__main__":
    main()
