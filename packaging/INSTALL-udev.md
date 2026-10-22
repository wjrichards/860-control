# Installing the udev rule

`g512d` needs read/write access to the keyboard's hidraw nodes. This rule grants that access to whoever is logged in at the local seat, and only for the G512 (`046d:c33c`).

```sh
sudo install -m 0644 packaging/70-g512-control.rules /etc/udev/rules.d/
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=hidraw
```

Then unplug and replug the keyboard, or log out and back in.

To check it worked, run the loop below. Your user should be listed with `rw-` on both keyboard nodes:

```sh
for n in /sys/class/hidraw/hidraw*; do
  grep -q 046D:0000C33C "$n/device/uevent" && getfacl -p "/dev/${n##*/}" | grep "^user:$USER"
done
```

To remove the rule, run `sudo rm /etc/udev/rules.d/70-g512-control.rules && sudo udevadm control --reload`, then replug the keyboard.

On Fedora Silverblue `/etc` is writable, so no `rpm-ostree` layering is needed.
