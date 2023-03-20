#! /usr/bin/python3

import os
import sys
import json
import urllib.parse as URL

symbolfile = sys.argv[1]
executable = sys.argv[2]

with open('scripts/vsc-launch.json', 'r') as f:
    config = json.load(f)

target_create_cmds = [
    f'target create --symfile {symbolfile} {executable}'
]

config['targetCreateCommands'] = target_create_cmds

config = json.dumps(config)
config = URL.quote(config)

resource = f'vscode://vadimcn.vscode-lldb/launch/config?{config}'

os.execvp('code', [
    'code',
    '--open-url',
    resource,
])
