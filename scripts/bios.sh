#!/bin/bash

BIOS="bios.bin"
BIOS_DIR="seabios/"
BIOS_REPO="https://github.com/exciting-kfs/seabios.git"
CROSS_PREFIX="i686-elf-"

if [ ! -f "$BIOS" ]; then
	echo "Error: $BIOS: file not found."

	if [ ! -d "$BIOS_DIR" ]; then
		echo "$BIOS: install bios."
		git clone "$BIOS_REPO"
	fi

	echo "$BIOS: compling..."
	make -C seabios/ CROSS_PREFIX=$CROSS_PREFIX > /dev/null 2> /dev/null
	echo "$BIOS: compile done."
	cp seabios/out/$BIOS .
fi