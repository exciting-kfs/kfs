#! /bin/bash

set -e

HDD=$1
HDD_SIZE_MB=64

SECTOR_SIZE=512
SECTOR_END=$(( $HDD_SIZE_MB * 1024 * 1024 / $SECTOR_SIZE ))

PART2_START=$(( $SECTOR_END / 2 ))
PART2_END=$(( $SECTOR_END - 1 ))

PART1_START=2048
PART1_END=$(( $PART2_START - 1 ))

echo CREATE $(basename $HDD) "($HDD_SIZE_MB""MB)"
qemu-img create -q -f qcow2 $HDD $HDD_SIZE_MB"M"
qemu-nbd $HDD --persistent &
NBD_SERVER=$!
trap "kill $NBD_SERVER" EXIT 

echo DOCKER-BUILD kfs-build-hdd-qcow
docker build -q -t kfs-build-hdd-qcow scripts/hdd > /dev/null

echo DOCKER-RUN kfs-build-hdd-qcow
docker run --rm -i kfs-build-hdd-qcow << EOF
add '' protocol:nbd server:host.docker.internal
run
part-init /dev/sda mbr
part-add /dev/sda p $PART1_START $PART1_END
part-add /dev/sda p $PART2_START $PART2_END
mkfs ext2 /dev/sda1
mkfs ext2 /dev/sda2
EOF
