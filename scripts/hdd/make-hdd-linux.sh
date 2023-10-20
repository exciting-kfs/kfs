#! /bin/bash

set -e

SYSROOT=$2
HDD=$1
HDD_SIZE_MB=512
HDD_NAME=$(basename $HDD)

SECTOR_SIZE=512
SECTOR_END=$(( $HDD_SIZE_MB * 1024 * 1024 / $SECTOR_SIZE ))

PART2_START=$(( $SECTOR_END / 2 ))
PART2_END=$(( $SECTOR_END - 1 ))

PART1_START=2048
PART1_END=$(( $PART2_START - 1 ))

BUILD_ROOT=/tmp/kfs-builder
NBD=/dev/nbd0
NBDP1=$NBD'p1'
NBDP2=$NBD'p2'

ssh -q kfs-builder "rm -rf $BUILD_ROOT && mkdir -p $BUILD_ROOT"

scp -q -r $SYSROOT "kfs-builder:$BUILD_ROOT/sysroot"

ssh -q kfs-builder bash << EOF

set -e

echo QEMU-IMG $HDD_NAME
qemu-img create -q -f qcow2 $BUILD_ROOT/$HDD_NAME $HDD_SIZE_MB"M"
qemu-nbd -f qcow2 -c $NBD $BUILD_ROOT/$HDD_NAME
trap "qemu-nbd -d $NBD > /dev/null" EXIT

echo FDISK $HDD_NAME
fdisk $NBD << EOCMD > /dev/null
n
p
1
$PART1_START
$PART1_END
n
p
2
$PART2_START
$PART2_END
w
EOCMD

echo MKFS.EXT2 $NBDP1
mkfs.ext2 -q $NBDP1 -b 1024

echo MKFS.EXT2 $NBDP2
mkfs.ext2 -q $NBDP2 -b 1024

mkdir -p $BUILD_ROOT/mnt/vol1
mount $NBDP1 $BUILD_ROOT/mnt/vol1

echo CP sysroot
cp -r $BUILD_ROOT/sysroot/* $BUILD_ROOT/mnt/vol1

mkdir -p $BUILD_ROOT/mnt/vol2
mount $NBDP2 $BUILD_ROOT/mnt/vol2

umount $BUILD_ROOT/mnt/vol1
umount $BUILD_ROOT/mnt/vol2

EOF

scp -q -r kfs-builder:$BUILD_ROOT/$HDD_NAME $HDD
