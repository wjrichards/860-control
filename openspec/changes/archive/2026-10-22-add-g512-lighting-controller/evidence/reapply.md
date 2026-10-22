# Re-apply timings (task 3.5)

Saved state during these checks: Static `#ffffff`, brightness 100. Service: installed `g512d.service`, started 02:01:12.

## Replug, 2026-10-22

The user re-enumerated the keyboard by writing to the USB `authorized` attribute; the kernel logged the parent hub `5-3` disconnecting as well. The user confirmed the keyboard came back solid white.

| Time | Source | Event |
|---|---|---|
| 02:02:21.386 | kernel | `usb 5-3.3: USB disconnect, device number 6` |
| 02:02:21.472 | g512d | `disconnected` (+86 ms) |
| 02:02:28.725 | kernel | `usb 5-3.3: new full-speed USB device number 9` |
| 02:02:29.077 | kernel | `hidraw1` created for interface 1 |
| 02:02:29.603 | g512d | `connected: ... on /dev/hidraw1`, saved state applied |

The saved state was applied **0.53 s after the lighting interface appeared** (0.88 s after USB enumeration). That is within the 3 s requirement. Most of the delay is the 500 ms settle wait for udev's `uaccess` ACL.

Two earlier "replugs" reported in the session left no kernel or service events (`/dev/hidraw1` unchanged since 01:48:23, USB device number 6). They are not counted.

## Resume from suspend, 2026-10-22

At the user's request, `systemctl suspend` was run from the session at 02:20:09.579. Afterwards `/sys/power/suspend_stats/success` went from 0 to 1.

| Time | Source | Event |
|---|---|---|
| 02:20:10.827 | kernel | `PM: suspend entry (deep)` |
| 02:20:30.573 | kernel | `PM: suspend exit` |
| 02:20:31.041 | systemd-sleep | `System returned from sleep operation 'suspend'` |
| 02:20:31.054 | g512d | `resumed from suspend; re-applying` (+0.48 s after kernel exit) |

The kernel logged no USB disconnect or re-enumeration for the keyboard on resume. The service re-applied through its existing handle, and logged no error or `disconnected`. That version of `g512d` did not log a successful re-apply, so the apply time is inferred: about 1 s settle after the resume line, so about 1.5 s after kernel exit. This is within the 3 s requirement but was not directly logged. Since 02:21:29, `g512d` logs `re-applied saved state (attempt N)` or a failure line after every hotplug or resume re-apply.

The user confirmed that the keyboard **went dark during sleep and was solid white after waking**. So the lighting was lost in suspend and restored by `g512d`.

## Second suspend: rule removed (not a resume failure)

Before this run, the user had already moved `/etc/udev/rules.d/70-g512-control.rules` to `/tmp` and re-enumerated the keyboard (the task 3.4 check). The kernel re-created `hidraw1` at 02:26:20.186 as `root 0600` with no user ACL.

| Time | Source | Event |
|---|---|---|
| 02:26:16.884 | g512d | `disconnected` |
| 02:26:21.629 | g512d | `could not re-apply saved state after 3 attempts` (hotplug path) |
| 02:26:32.497 | kernel | `PM: suspend entry (deep)` (`systemctl suspend` run from the session with the user's OK) |
| 02:26:50.066 | kernel | `PM: suspend exit` |
| 02:26:50.535 | g512d | `resumed from suspend; re-applying` (+0.47 s) |
| 02:26:52.536 | g512d | `could not re-apply saved state after 3 attempts` |

This is the expected result with no device access. The service stays up and reports `permission-denied`; see the device states below. Since 02:27:49 the failure line names the state, for example `(keyboard permission-denied)`.

# Device states (task 3.4)

- **`permission-denied`:** after the rule was removed and the keyboard re-enumerated, `busctl --user get-property ... DeviceState` returned `"permission-denied"`. A freshly started `g512d` (02:27:49) reported the same, and the install script printed the udev install hint.
- **`disconnected`:** during the 02:02:21–02:02:29 re-enumeration, `g512d` logged `disconnected` 86 ms after the kernel disconnect, and `connected` once the node returned. The D-Bus property wasn't sampled inside that window. It is computed from the same state the log line reports.

## Rule restored, 2026-10-22

The user put the rule back and re-enumerated the keyboard. The ACL now gives `user:wrichards:rw-` again. Both re-enumerations logged the explicit success line:

| Kernel `hidraw1` created | g512d `re-applied saved state (attempt 1)` | Delay |
|---|---|---|
| 02:30:10.027 | 02:30:10.556 | 0.53 s |
| 02:30:28.465 | 02:30:28.992 | 0.53 s |
