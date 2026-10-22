# Tasks

## 1. Device access and protocol check

- [x] 1.1 Add `packaging/70-g512-control.rules` (rule from design.md) and an install note; after the user installs it and replugs, verify `getfacl /dev/hidraw1` lists the session user with `rw` and the mouse's hidraw nodes are unchanged
- [x] 1.2 Write-command check: with a throwaway script, send `setZoneEffect` by hand for Static red, Breathe, Cycle, Wave (`0x0004`) and Ripple (RAM-only); record working byte layouts, which effects take a colour, and the usable period range in `evidence/set-zone-effect.md`; update design.md's effect table and the device-protocol spec if anything differs; verify by replugging that the keyboard returns to its default lighting

## 2. Protocol crate (`g512-hidpp`)

- [x] 2.1 Create the Cargo workspace with `g512-hidpp`, `g512d` and `g512-control` crates; verify `cargo build -p g512-hidpp -p g512d` succeeds on the host with no C dependencies (`cargo tree` shows no `-sys` crates)
- [x] 2.2 Implement the `Feature` allowlist enum and request builder; verify a unit test asserts the enum holds exactly `0x0000, 0x0001, 0x0003, 0x0005, 0x8070, 0x8080`
- [x] 2.3 Implement device discovery from sysfs (USB ID + `0xFF43` usage page); verify unit tests using the recorded descriptors in `evidence/feature-table.txt` select interface 1 and reject interface 0 and a mouse descriptor
- [x] 2.4 Implement the transport with reply matching, HID++ error decoding, skipping unrelated reports, and the 1 s timeout over a `Transport` trait; verify unit tests with a fake transport cover a normal reply, an error reply, an interleaved key report, and a timeout
- [x] 2.5 Implement feature resolution, device name / firmware reads, and effect discovery; verify a replay test built from `evidence/effects-out.txt` yields Off, Static, Breathe, Cycle, Wave, Ripple selectable and `0x0005` unrecognised, and a test with a moved `0x8070` slot still resolves
- [x] 2.6 Implement `setZoneEffect` using the layout recorded in task 1.2, RAM-only; verify unit tests check the exact bytes for each effect against `evidence/set-zone-effect.md`, and a hardware test (`cargo test -- --ignored`) sets Static green on the real keyboard

## 3. Service (`g512d`)

- [x] 3.1 Implement lighting state, validation and brightness scaling; verify unit tests cover the spec's `#FF8040` at 50% → `#804020`, brightness 0 → Off, and rejection of brightness 150 and of an unreported effect
- [x] 3.2 Implement the state file (atomic write, load on start, pending flag); verify unit tests round-trip a state and show a corrupt file is ignored with a logged warning
- [x] 3.3 Implement the D-Bus interface `io.github.g512control.Lighting1` (device state, effects with parameters, get/set state, change signal) with single-instance name ownership; verify with `busctl --user introspect` and `busctl --user call` that set/get works and that a second `g512d` exits with a message
- [ ] 3.4 Implement device states (`connected`, `disconnected`, `permission-denied`, `unsupported`); verify by running without the udev rule (`permission-denied`) and with the keyboard unplugged (`disconnected`)
- [ ] 3.5 Implement hotplug (netlink uevents) and resume (logind `PrepareForSleep`) reapply with retries; verify that replugging and `systemctl suspend` each restore the saved state within 3 s, and record the timings in `evidence/reapply.md`
- [ ] 3.6 Add the systemd user unit, D-Bus service file and `packaging/install-service.sh`; verify that after running it, `systemctl --user status g512d` is active and the saved state is applied at next login

## 4. App (`g512-control`)

- [ ] 4.1 Add the Flatpak manifest (`org.gnome.Platform//50`, Rust extension, `--talk-name` only, no `--device`); verify `flatpak run --command=ls io.github.g512control.Control /dev` shows no hidraw nodes and the app starts
- [ ] 4.2 Implement the D-Bus client and the status pages (service missing, `permission-denied` with the udev install command, `disconnected`, `unsupported`); verify each page appears by stopping the service, removing the rule, and unplugging the keyboard
- [ ] 4.3 Implement the main page: effect row, colour row and speed row shown per the effect's parameters, and the brightness row; verify by selecting each effect that the controls match the spec's Cycle and Static scenarios
- [ ] 4.4 Implement live apply with 100 ms throttle and trailing send, toast and rollback on rejection, and following external change signals without echoing them back; verify by counting calls in `busctl --user monitor` while dragging brightness, and by setting Ripple with `busctl` while the app is open
- [ ] 4.5 Check GNOME conventions: light/dark and accent follow the system, keyboard-only use works, and every control has an accessible name; verify by toggling dark style and accent in Settings, doing a full keyboard-only pass, and inspecting names with the GTK inspector (`GTK_DEBUG=interactive`)

## 5. Wrap-up

- [ ] 5.1 Write README (install service + udev rule + Flatpak, uninstall/rollback, "don't run alongside OpenRGB/g810-led"); verify a fresh install following only the README works on this machine
- [ ] 5.2 Run `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` and `openspec validate add-g512-lighting-controller --strict`; verify all pass
