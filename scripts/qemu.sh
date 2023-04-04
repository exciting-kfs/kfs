#! /bin/bash

if [ $# -lt 3 ]; then
	echo 'Usage: qemu.sh "ISO file" "kernbuf serial" "unit_test serial" ...extraflags'; exit 1
fi

RESCUE="$1"
shift

SERIAL="$1"
shift

UNIT_TEST="$1"
shift

trap "rm -f $SERIAL $UNIT_TEST" EXIT

until [ -p $UNIT_TEST ] && [ -p $SERIAL ]
do
    sleep 1
done

# -m 3968(4096 - 128): almost maximum memory in x86 (without PAE)
qemu-system-i386                    \
    -machine pc,max-ram-below-4g=4G \
    -m 3968                         \
    -boot d                         \
    -vga std                        \
    -cdrom $RESCUE                  \
    -serial pipe:$SERIAL            \
    -serial pipe:$UNIT_TEST         \
    $@
