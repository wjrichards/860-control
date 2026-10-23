"""Drive the G512 Control window over AT-SPI.

  a11y_drive.py dump                 visible nodes (role: name [= value] [checked])
  a11y_drive.py click NAME...        run the first action of each node named NAME
  a11y_drive.py clickrole ROLE NAME  same, limited to one role (e.g. "toggle button")
  a11y_drive.py burst N NAME         click NAME N times as fast as possible
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


def walk(node, depth=0, out=None):
    out = [] if out is None else out
    if depth > 40:
        return out
    try:
        states = node.get_state_set()
        if depth and not states.contains(Atspi.StateType.SHOWING):
            return out
        out.append((depth, node, states))
        for i in range(node.get_child_count()):
            walk(node.get_child_at_index(i), depth + 1, out)
    except Exception:
        pass
    return out


def describe(node, states):
    name, role = node.get_name() or "", node.get_role_name()
    extra = ""
    try:
        v = node.get_value_iface()
        if v:
            extra += f" = {v.get_current_value():.0f}"
    except Exception:
        pass
    if states.contains(Atspi.StateType.CHECKED) or states.contains(Atspi.StateType.PRESSED):
        extra += " [checked]"
    return f"{role}: {name}{extra}"


def has_action(node):
    try:
        a = node.get_action_iface()
        return bool(a) and a.get_n_actions() > 0
    except Exception:
        return False


def find(name, role=None):
    """The first matching node with an action, else the first match."""
    matches = [n for _, n, _ in walk(app())
               if (n.get_name() or "") == name and (role is None or n.get_role_name() == role)]
    return next((n for n in matches if has_action(n)), matches[0] if matches else None)


def act(node):
    a = node.get_action_iface()
    if not a or a.get_n_actions() == 0:
        raise SystemExit(f"{node.get_name()!r} has no action")
    a.do_action(0)


def main():
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "dump":
        for depth, node, states in walk(app()):
            text = describe(node, states)
            if text.split(": ", 1)[1] or "=" in text:
                print("  " * min(depth, 10) + text)
    elif cmd == "click":
        for name in args:
            node = find(name)
            if node is None:
                sys.exit(f"no visible node named {name!r}")
            act(node)
            time.sleep(0.3)
    elif cmd == "clickrole":
        node = find(args[1], args[0])
        if node is None:
            sys.exit(f"no visible {args[0]} named {args[1]!r}")
        act(node)
    elif cmd == "burst":
        node = find(args[1])
        if node is None:
            sys.exit(f"no visible node named {args[1]!r}")
        gap = float(args[2]) / 1000 if len(args) > 2 else 0.0
        t = time.monotonic()
        for _ in range(int(args[0])):
            act(node)
            time.sleep(gap)
        print(f"{args[0]} clicks in {(time.monotonic() - t) * 1000:.0f} ms")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
