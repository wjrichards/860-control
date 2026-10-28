"""Task 3.2: look at the app's Mouse Buttons group and Change dialog over
AT-SPI. Nothing here applies a change.

  app_buttons_a11y.py rows        the group's rows: role, name, description
  app_buttons_a11y.py act NAME    run the first action of the button called NAME
  app_buttons_a11y.py dialog      the open dialog's options and buttons

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
    if depth > 80:
        return out
    try:
        out.append(node)
        for i in range(node.get_child_count()):
            walk(node.get_child_at_index(i), out, depth + 1)
    except Exception:
        pass
    return out


def visible(n):
    return n.get_state_set().contains(Atspi.StateType.VISIBLE)


def group_nodes():
    """Nodes from the "Mouse Buttons" group heading to the next group."""
    out, inside = [], False
    for n in walk(app(), []):
        role, name = n.get_role_name(), n.get_name() or ""
        if role == "grouping" and name and inside and name != "Mouse Buttons":
            break
        if role == "grouping" and name == "Mouse Buttons":
            inside = True
        if inside:
            out.append(n)
    return out


def flags(n):
    s = n.get_state_set()
    pairs = (("checked", Atspi.StateType.CHECKED), ("sensitive", Atspi.StateType.SENSITIVE))
    return " ".join(f for f, t in pairs if s.contains(t))


def describe(n):
    role, name = n.get_role_name(), n.get_name() or ""
    desc = n.get_description() or ""
    return f"{role}: {name}" + (f" — {desc}" if desc else "") + f" [{flags(n)}]"


def main():
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "rows":
        nodes = [n for n in group_nodes() if visible(n)]
        if not nodes:
            sys.exit("no Mouse Buttons group (hidden?)")
        for n in nodes:
            if n.get_role_name() in ("grouping", "list item", "button", "row") and (n.get_name() or ""):
                print(describe(n))
    elif cmd == "act":
        for n in walk(app(), []):
            a = n.get_action_iface()
            if (n.get_name() or "") == args[0] and n.get_role_name() == "button" and visible(n) and a and a.get_n_actions():
                a.do_action(0)
                print(f"ran {a.get_action_name(0)!r} on {n.get_role_name()} {args[0]!r}")
                return
        sys.exit(f"no actionable {args[0]!r}")
    elif cmd == "dialog":
        dialogs = [n for n in walk(app(), []) if n.get_role_name() in ("dialog", "alert") and visible(n)]
        if not dialogs:
            sys.exit("no dialog open")
        for d in dialogs:
            print(describe(d))
            for n in walk(d, [])[1:]:
                role = n.get_role_name()
                if role in ("radio button", "check box", "button", "list item", "grouping", "row") and visible(n):
                    if n.get_name():
                        print("  " + describe(n))


if __name__ == "__main__":
    main()
