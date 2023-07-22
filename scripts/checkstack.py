#! /usr/bin/python3

import sys
import re

disas = sys.stdin.read()
disas = disas.split('\n\n')

functs = []

for section in disas:
    lines = section.split('\n')
    if len(lines) and re.match(r'[a-f0-9]+ <', lines[0]):

        stack_usage = 0
        for instruction in lines[1:]:
            hex_digits = re.search(r'sub\s+esp,0x([a-f0-9]+)', instruction)
            if hex_digits:
                hex_digits = int(hex_digits.group(1), 16)
                if hex_digits > stack_usage:
                    stack_usage = hex_digits

        name = re.search(r'<(.*)>:', lines[0]).group(1)

        functs.append({
            'name': name,
            'stack': stack_usage,
        })

functs.sort(key=lambda x: x['stack'], reverse=True)

for funct in functs:
    name = funct['name']
    stack = funct['stack']
    print(f'{name}: {stack} bytes.')
