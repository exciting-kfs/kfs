#! /bin/bash

set -o errexit

if [ -z "$I386_GRUB2_PREFIX" ]; then
    echo 'Error: grub2 prefix not found.'
    echo 'To build rescue image, install prefix of grub2 is needed.'
    echo 'Please set $I386_GRUB2_PREFIX appropriate loaction.'
    exit 1
fi

GRUB2_MKRESCUE=$I386_GRUB2_PREFIX/bin/grub-mkrescue
GRUB2_I386_LIB=$I386_GRUB2_PREFIX/lib/grub/i386-pc

if [ ! -f "$GRUB2_MKRESCUE" ]; then
    echo 'Error: grub-mkrescue: command not found.'
    echo 'Please double check $I386_GRUB2_PREFIX is properly configured.'
    echo "(currently $I386_GRUB2_PREFIX)"
    exit 1
fi

if [ ! -d "$GRUB2_I386_LIB" ]; then
    echo 'Error: cannot find grub2 i386 library.'
    echo 'Please double check $I386_GRUB2_PREFIX is properly configured.'
    echo "(currently $I386_GRUB2_PREFIX)"
    echo 'and grub2 was installed with i386 support.'
    exit 1
fi

KERNEL_BIN=$(scripts/build_get_executable.sh)

if [ -z "$KERNEL_BIN" ]; then
    echo 'Error: build failed.'
    exit 1
fi

ISO_ROOT=target/iso

mkdir -p $ISO_ROOT/boot/grub
cp scripts/grub.cfg $ISO_ROOT/boot/grub
cp "$KERNEL_BIN" $ISO_ROOT/boot

"$GRUB2_MKRESCUE" -d "$GRUB2_I386_LIB" "$ISO_ROOT" -o rescue.iso 2>/dev/null  > /dev/null
