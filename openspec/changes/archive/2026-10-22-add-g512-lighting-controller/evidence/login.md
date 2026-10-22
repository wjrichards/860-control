# Restore at login (task 3.6), 2026-10-22

**Not checked across a real login.** Two attempts did not end the session. Session 2 (from 06:57), the user manager and `g512d` PID 454651 (from 09:41) were all unchanged, and this Claude Code session, which runs inside the login session, kept running. The user chose to tick 3.6 on the evidence below. The first real check will be the next login or reboot.

What the tick rests on:

- **The service is installed and active.** After `packaging/install-service.sh`, `systemctl --user status g512d` shows `active (running)`, `enabled` (see `install.md`).
- **It starts at login.** The unit has `WantedBy=default.target`, and install linked it into `~/.config/systemd/user/default.target.wants/`. Linger is off, so the user manager, and with it `g512d`, starts at each login.
- **It applies the saved state at startup.** After the fresh reinstall, the service started without any SetState from anyone. It connected (`connected: ... on /dev/hidraw1`), and `State` read the saved `"Ripple" 61 56 70 52 100`. Startup takes the same connect-and-apply path as a replug, and that path was measured at 0.53 s after the hidraw node appeared (`reapply.md`).
