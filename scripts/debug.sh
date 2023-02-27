#! /bin/bash

scripts/bios.sh &&		\
scripts/rescue.sh &&		\
qemu-system-i386 		\
	-boot d			\
	-vga std 		\
	-cdrom rescue.iso	\
	-m 4096M 		\
	-bios bios.bin		\
	-s -S