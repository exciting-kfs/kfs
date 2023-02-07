#! /bin/bash

scripts/rescue.sh && qemu-system-i386 -boot d -vga std -cdrom rescue.iso
