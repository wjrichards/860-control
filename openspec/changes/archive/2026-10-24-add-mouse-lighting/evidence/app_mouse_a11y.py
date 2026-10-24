"""Task 3.1: look at and drive the app's Mouse section over AT-SPI.

  app_mouse_a11y.py rows                  the Mouse section's rows: role, name, visible, checked
  app_mouse_a11y.py act NAME [ROLE]       scroll NAME into view and run its first action
  app_mouse_a11y.py set SLIDER VALUE      set a slider in the Mouse section
  app_mouse_a11y.py pick ITEM             choose ITEM in the Effect list (GTK 4's combo row
                                          exposes no action, so this fails; kept for reference)

Unlike the per-key driver, this walks nodes that are scrolled out of view
too (the Mouse group sits below the fold), using VISIBLE rather than SHOWING.
"""
import sys
import time

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
    if depth > 60:
        return out
    try:
        out.append(node)
        for i in range(node.get_child_count()):
            walk(node.get_child_at_index(i), out, depth + 1)
    except Exception:
        pass
    return out


def states(node):
    s = node.get_state_set()
    return {n for n, t in (("visible", Atspi.StateType.VISIBLE), ("showing", Atspi.StateType.SHOWING),
                           ("checked", Atspi.StateType.CHECKED), ("sensitive", Atspi.StateType.SENSITIVE))
            if s.contains(t)}


def mouse_rows(nodes):
    """Nodes from the "Mouse" group heading up to the end of the page."""
    out, inside = [], False
    for n in nodes:
        name, role = n.get_name() or "", n.get_role_name()
        if role == "grouping" and name == "Mouse":
            inside = True
        if inside:
            out.append(n)
    return out


def act(node):
    try:
        node.get_component_iface().scroll_to(Atspi.ScrollType.ANYWHERE)
    except Exception:
        pass
    a = node.get_action_iface()
    if not a or a.get_n_actions() == 0:
        sys.exit(f"{node.get_name()!r} has no action")
    a.do_action(0)


def main():
    cmd, args = sys.argv[1], sys.argv[2:]
    nodes = walk(app(), [])
    rows = mouse_rows(nodes)
    if cmd == "rows":
        if not rows:
            print("no Mouse group")
        for n in rows:
            st = states(n)
            if n.get_role_name() in ("grouping", "switch", "combo box", "button", "slider", "label", "list item",
                                     "push button", "toggle button") and n.get_name():
                flags = " ".join(sorted(st & {"visible", "checked"}))
                print(f"{n.get_role_name()}: {n.get_name()} [{flags}]")
    elif cmd == "act":
        name, role = args[0], (args[1] if len(args) > 1 else None)
        for n in rows:
            if (n.get_name() or "") == name and (role is None or n.get_role_name() == role):
                a = n.get_action_iface()
                if a and a.get_n_actions():
                    act(n)
                    return
        sys.exit(f"no actionable {name!r} in the Mouse group")
    elif cmd == "set":
        name, value = args[0], float(args[1])
        for n in rows:
            if (n.get_name() or "") == name and n.get_role_name() == "slider":
                n.get_value_iface().set_current_value(value)
                return
        sys.exit(f"no slider {name!r} in the Mouse group")
    elif cmd == "pick":
        effect = [n for n in rows if n.get_role_name() == "combo box" or (n.get_name() or "") == "Effect"]
        target = next((n for n in effect if n.get_action_iface() and n.get_action_iface().get_n_actions()), None)
        if not target:
            sys.exit("no Effect row with an action")
        act(target)
        time.sleep(0.6)
        for n in walk(app(), []):
            if (n.get_name() or "") == args[0] and n.get_role_name() in ("list item", "label", "menu item") \
                    and "showing" in states(n):
                parent = n
                while parent is not None:
                    a = parent.get_action_iface()
                    if a and a.get_n_actions():
                        a.do_action(0)
                        return
                    parent = parent.get_parent()
        sys.exit(f"no {args[0]!r} item in the open list")


if __name__ == "__main__":
    main()
