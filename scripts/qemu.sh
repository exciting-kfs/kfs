#! /bin/bash

trap 'SIGNALED=1' INT

if [ $# -lt 2 ]; then
	echo 'Usage: qemu.sh "ISO file" "kernbuf serial" ...extraflags'; exit 1
fi

RESCUE="$1"
shift

COM1="$1"
shift

# -m 4032(4096 - 64): almost maximum memory in x86 (without PAE)
qemu-system-i386                    \
    -cpu max			    \
    -machine pc,max-ram-below-4g=4G \
    -m 4000                         \
    -boot d                         \
    -vga std                        \
    -device isa-debug-exit          \
    -cdrom $RESCUE                  \
    -serial $COM1                   \
    -action reboot=shutdown         \
    $@

RESULT=$?
if [ \( $RESULT -eq 0 \) -a ! "$SIGNALED" ]; then
    echo "[!] Automatic shutdown detected. (triple fault?)"
    exit 1
fi

RESULT=$(( $RESULT / 2 ))
if [ $RESULT -ne 0 ]; then
    echo "[!] Kernel Panic detected. (code=$RESULT)"
    exit 1
fi

exit 0
