#!/bin/sh
# Call a method on the fake service: sh target/fake_call.sh SetState Static "(38, 162, 105)" 50 100
m=$1; shift
exec gdbus call --session -d io.github.g512control.Lighting1Test -o /io/github/g512control/Lighting1 -m io.github.g512control.Lighting1.$m "$@"
