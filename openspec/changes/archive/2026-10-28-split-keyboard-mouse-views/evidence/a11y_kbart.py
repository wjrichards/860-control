"""Find the keyboard illustration button, print its name, activate it, and
report whether the Key Maps page opened."""
import sys
import time

sys.path.insert(0, "/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/keyboard-mouse-views-7d1c/target")
import a11y_views as v  # noqa: E402

nodes = v.walk(v.app(), [])
art = [n for n in nodes if n.get_role_name() == "button" and (n.get_name() or "").startswith("G512 keyboard")]
if not art:
    sys.exit("no keyboard illustration button")
print("name:", art[0].get_name())
art[0].get_action_iface().do_action(0)
time.sleep(1)
names = {(n.get_role_name(), n.get_name() or "") for n in v.walk(v.app(), []) if v.showing(n)}
print("Key Maps open:", any("Key Maps" in name or name == "Map being edited" for _, name in names))
