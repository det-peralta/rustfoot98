#!/bin/bash
# usage: c.sh '<json>'   appends a command and waits for its result
n=$(wc -l < analysis/cmd.out); echo "$1" >> analysis/cmd.jsonl
for i in $(seq 1 150); do [ "$(wc -l < analysis/cmd.out)" -gt "$n" ] && break; sleep 0.4; done
tail -n +$((n+1)) analysis/cmd.out
