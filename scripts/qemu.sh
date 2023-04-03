#! /bin/bash

if [ $# -lt 2 ]; then
	echo 'Usage: qemu.sh "ISO file" "serial backend" ...extraflags'; exit 1
fi

RESCUE="$1"
shift

SERIAL="$1"
shift

trap "rm -f $SERIAL" EXIT

if [ -p $SERIAL ]; then
    rm -f $SERIAL
fi

mkfifo $SERIAL
mkfifo /tmp/serial1

# -m 3968(4096 - 128): almost maximum memory in x86 (without PAE)
qemu-system-i386                    \
    -machine pc,max-ram-below-4g=4G \
    -m 3968                         \
    -boot d                         \
    -vga std                        \
    -cdrom $RESCUE                  \
    -serial pipe:$SERIAL            \
    -serial pipe:/tmp/serial1            \
    $@
