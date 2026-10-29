#!/bin/sh
# Task 6.2 / 7.4: turn GNOME's animations off for a few seconds, run the app
# on Breathe with the tick log, and put the setting back as it was. This
# changes the desktop's own setting, so run it only with the user's OK.
# (GTK in the GNOME 50 runtime reads enable-animations only through the
# settings portal; a private GSettings keyfile with portals off doesn't reach it.)
cd "$(dirname "$0")/.." || exit 1
was=$(gsettings get org.gnome.desktop.interface enable-animations)
trap 'gsettings set org.gnome.desktop.interface enable-animations "$was"' EXIT
for anim in false true; do
    gsettings set org.gnome.desktop.interface enable-animations "$anim"
    PRE='sh target/fake_call.sh SetState Breathe "(38, 162, 105)" 50 100' TICKS=1 \
    SHOT=$PWD/target/anim-$anim.png VIEW=keyboard sh target/run_app_fake.sh > /dev/null
    sleep 5
    echo "enable-animations=$anim:"
    grep -E "ticks" target/app.log | tail -2
done
