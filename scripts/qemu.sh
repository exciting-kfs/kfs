#! /bin/bash

trap 'SIGNALED=1' INT

if [ $# -lt 3 ]; then
	echo 'Usage: qemu.sh "ISO file" "HDD file" "kernbuf serial" ...extraflags'; exit 1
fi

RESCUE="$1"
shift

HDD="$1"
shift

COM1="$1"
shift

# -m 4032(4096 - 64): almost maximum memory in x86 (without PAE)
qemu-system-i386                                                \
    -cpu max                                                    \
    -smp sockets=1,cores=4,threads=1                            \
    -machine pc,max-ram-below-4g=4G                             \
    -m 4000                                                     \
    -vga std                                                    \
    -drive file=$RESCUE,if=none,format=raw,id=rescue            \
    -device ide-hd,drive=rescue,bus=ide.1,unit=0,bootindex=1    \
    -drive file=$HDD,if=none,format=qcow2,id=hdd                \
    -device ide-hd,drive=hdd,bus=ide.0,unit=0                   \
    -device isa-debug-exit                                      \
    -action reboot=shutdown                                     \
    -serial $COM1                                               \
    $@ | tee log/log-"$(date "+%m.%d-%H:%M:%S")"

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
