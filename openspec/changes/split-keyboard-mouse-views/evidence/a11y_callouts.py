"""Check the mouse callouts over AT-SPI: list them with their sensitivity,
activate G9's, and report the dialog that opens.

  a11y_callouts.py            list callouts, open G9's dialog, report it, close it
"""
import sys
import time

sys.path.insert(0, "/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/keyboard-mouse-views-7d1c/target")
import a11y_views as v  # noqa: E402
from a11y_views import Atspi  # noqa: E402

nodes = v.walk(v.app(), [])
art = [n for n in nodes if (n.get_name() or "").startswith("G502 HERO mouse")]
print("drawing:", [(n.get_role_name(), n.get_name()) for n in art])
seen = set()
callouts = []
for n in nodes:
    name = n.get_name() or ""
    if n.get_role_name() == "button" and name[:2].strip().isdigit() and name not in seen:
        seen.add(name)
        callouts.append(n)
for n in callouts:
    sensitive = n.get_state_set().contains(Atspi.StateType.SENSITIVE)
    print(f"  {n.get_name()!r:45} sensitive={sensitive} desc={n.get_description()!r}")
print("callouts:", len(callouts))
g9 = [n for n in callouts if (n.get_name() or "").startswith("6 G9")]
if not g9:
    sys.exit("no G9 callout")
g9[0].get_action_iface().do_action(0)
for _ in range(10):
    time.sleep(0.5)
    try:
        dialogs = [n for n in v.walk(v.app(), []) if n.get_role_name() == "dialog" and v.showing(n)]
        break
    except SystemExit:
        dialogs = []
for d in dialogs:
    names = sorted({n.get_name() or n.get_parent().get_name() for n in v.walk(d, []) if n.get_role_name() in ("check box", "radio button") and n.get_state_set().contains(Atspi.StateType.CHECKED)})
    print("dialog:", d.get_name(), "checked:", names)
    cancel = [n for n in v.walk(d, []) if n.get_role_name() == "button" and n.get_name() == "Cancel"]
    if cancel:
        cancel[0].get_action_iface().do_action(0)
if not dialogs:
    print("dialog: none")
