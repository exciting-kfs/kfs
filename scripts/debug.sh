#! /bin/bash

scripts/rescue.sh && qemu-system-i386 -s -S -boot d -vga std -cdrom rescue.iso