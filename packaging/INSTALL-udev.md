# Installing the udev rule

`g512d` needs read/write access to the keyboard's and mouse's hidraw nodes. This rule grants that access to whoever is logged in at the local seat, and only for the G512 keyboard (`046d:c33c`) and the G502 HERO mouse (`046d:c08b`).

If you installed the rule before mouse support, install it again with the same commands. The new version adds one line for the mouse.

```sh
sudo install -m 0644 packaging/70-g512-control.rules /etc/udev/rules.d/
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=hidraw
```

`udevadm trigger` applies it to devices that are already plugged in. Otherwise, unplug and replug them, or log out and back in.

To check it worked, run the loop below. Your user should be listed with `rw-` on both keyboard nodes and both mouse nodes:

```sh
for n in /sys/class/hidraw/hidraw*; do
  grep -q -E '046D:0000(C33C|C08B)' "$n/device/uevent" && getfacl -p "/dev/${n##*/}" | grep "^user:$USER"
done
```

To remove the rule, run `sudo rm /etc/udev/rules.d/70-g512-control.rules && sudo udevadm control --reload`, then replug the keyboard and mouse.

On Fedora Silverblue `/etc` is writable, so no `rpm-ostree` layering is needed.
