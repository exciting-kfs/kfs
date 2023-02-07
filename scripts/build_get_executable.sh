#! /bin/bash

NAME=kernel

read -r -d '' SCRIPT << EOSCRIPT
import sys
import json

for line in sys.stdin.readlines():
	p = json.loads(line)
	try:
		if p['target']['name'] == '$NAME':
			print(p['executable'])
			exit(0)
	except KeyError:
		continue
exit(1)
EOSCRIPT

cargo build --message-format=json | python3 -c "$SCRIPT"