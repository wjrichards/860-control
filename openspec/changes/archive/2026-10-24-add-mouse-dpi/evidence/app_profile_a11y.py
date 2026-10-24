"""Task 3.1: look at and drive the app's Mouse Profile group over AT-SPI.

  app_profile_a11y.py rows                 the group's rows: role, name, value, flags
  app_profile_a11y.py set STEP VALUE       set step STEP's spin button (1-based)
  app_profile_a11y.py act NAME             run the first action of NAME in the group

Walks nodes scrolled out of view too, using VISIBLE rather than SHOWING.
"""
import sys

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402


def app():
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        a = desktop.get_child_at_index(i)
        if a and (a.get_name() or "") == "g512-control":
            return a
    sys.exit("g512-control is not on the accessibility bus")


def walk(node, out, depth=0):
    if depth > 70:
        return out
    try:
        out.append(node)
        for i in range(node.get_child_count()):
            walk(node.get_child_at_index(i), out, depth + 1)
    except Exception:
        pass
    return out


def group_nodes():
    """Nodes from the "Mouse Profile" group heading to the next group."""
    out, inside = [], False
    for n in walk(app(), []):
        role, name = n.get_role_name(), n.get_name() or ""
        if role == "grouping" and name and inside and name != "Mouse Profile":
            break
        if role == "grouping" and name == "Mouse Profile":
            inside = True
        if inside:
            out.append(n)
    return out


def value(n):
    try:
        v = n.get_value_iface()
        return f" = {v.get_current_value():.0f}" if v else ""
    except Exception:
        return ""


def flags(n):
    s = n.get_state_set()
    return " ".join(f for f, t in (("checked", Atspi.StateType.CHECKED), ("sensitive", Atspi.StateType.SENSITIVE))
                    if s.contains(t))


def main():
    cmd, args = sys.argv[1], sys.argv[2:]
    nodes = group_nodes()
    if not nodes:
        sys.exit("no Mouse Profile group (hidden?)")
    if cmd == "rows":
        for n in nodes:
            role, name = n.get_role_name(), n.get_name() or ""
            if role in ("combo box", "spin button", "check box", "radio button", "button", "list item", "label") and name:
                print(f"{role}: {name}{value(n)} [{flags(n)}]")
    elif cmd == "set":
        step, val = int(args[0]), float(args[1])
        spins = [n for n in nodes if n.get_role_name() == "spin button"]
        spins[step - 1].get_value_iface().set_current_value(val)
    elif cmd == "act":
        for n in nodes:
            a = n.get_action_iface()
            if (n.get_name() or "") == args[0] and a and a.get_n_actions():
                a.do_action(0)
                return
        sys.exit(f"no actionable {args[0]!r}")


if __name__ == "__main__":
    main()
