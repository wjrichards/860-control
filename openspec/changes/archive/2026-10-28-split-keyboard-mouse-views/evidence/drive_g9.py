"""Drive G9 over AT-SPI for the motion recording: open its dialog from its
callout, pick Disabled and Apply (the confirm pulse). GTK 4 doesn't
support AT-SPI grab_focus, so the hover is checked by hand."""
import sys
import time

sys.path.insert(0, "/var/home/wrichards/Projects/Software/g512-control/.claude/worktrees/keyboard-mouse-views-7d1c/target")
import a11y_views as v  # noqa: E402


def find(pred):
    for _ in range(20):
        try:
            hits = [n for n in v.walk(v.app(), []) if pred(n)]
        except Exception:
            hits = []
        if hits:
            return hits[0]
        time.sleep(0.25)
    sys.exit("not found")


def g9(n):
    return n.get_role_name() == "button" and (n.get_name() or "").startswith("6 G9")


time.sleep(float(sys.argv[1]) if len(sys.argv) > 1 else 2.0)
find(g9).get_action_iface().do_action(0)
time.sleep(1.0)
def disabled(n):
    if n.get_role_name() not in ("radio button", "check box"):
        return False
    parent = n.get_parent()
    return "Disabled" in (n.get_name(), parent.get_name() if parent else None)


find(disabled).get_action_iface().do_action(0)
time.sleep(0.3)
find(lambda n: n.get_role_name() == "button" and n.get_name() == "Apply").get_action_iface().do_action(0)
print(f"{time.monotonic():.2f} applied Disabled to G9")
