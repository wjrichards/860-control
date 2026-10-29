#!/bin/sh
# Set a property on the fake through its Test interface: sh target/fake_set.sh MouseState "'disconnected'"
exec gdbus call --session -d io.github.g512control.Lighting1Test -o /io/github/g512control/Lighting1 \
    -m io.github.g512control.Test.Set "$1" "<$2>"
