#! /bin/bash

SERIAL=/tmp/serial0

trap "rm -f $SERIAL" EXIT

if [ -p $SERIAL ]; then
	rm -f $SERIAL
fi

mkfifo $SERIAL

scripts/rescue.sh && qemu-system-i386	\
 -boot d								\
 -vga std								\
 -cdrom rescue.iso						\
 -monitor stdio							\
 -serial pipe:$SERIAL

