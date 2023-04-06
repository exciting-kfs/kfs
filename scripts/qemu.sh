#! /bin/bash

if [ $# -lt 3 ]; then
	echo 'Usage: qemu.sh "ISO file" "kernbuf serial" "unit_test serial" ...extraflags'; exit 1
fi

RESCUE="$1"
shift

COM1="$1"
shift

COM2="$1"
shift

trap "rm -f $COM1 $COM2" EXIT

until [ -p $COM2 ] && [ -p $COM1 ]
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
    -serial pipe:$COM1              \
    -serial pipe:$COM2              \
    $@
