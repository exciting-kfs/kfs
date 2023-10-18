#! /bin/bash

set -e

SYSROOT=$2
HDD=$1
HDD_SIZE_MB=512

SECTOR_SIZE=512
SECTOR_END=$(( $HDD_SIZE_MB * 1024 * 1024 / $SECTOR_SIZE ))

PART2_START=$(( $SECTOR_END / 3 ))
PART2_END=$(( $SECTOR_END - 1 ))

PART1_START=2048
PART1_END=$(( $PART2_START - 1 ))

if [ $(uname -s) = 'Darwin' ]; then
    MAX_RETRY=10
    ATTEMPT=1
    until docker ps > /dev/null 2> /dev/null || [ $MAX_RETRY -le $ATTEMPT ] ; do
        echo TRY-OPEN docker "($ATTEMPT / $MAX_RETRY)"
        open -g -a docker
        ATTEMPT=$(( $ATTEMPT + 1 ))
        sleep 5
    done
fi

if ! docker ps > /dev/null 2> /dev/null; then
    echo error: docker daemon is not running.
    exit 1
fi

echo QEMU-IMG $(basename $HDD) "($HDD_SIZE_MB""MB)"
qemu-img create -q -f qcow2 $HDD $HDD_SIZE_MB"M"
qemu-nbd --persistent $HDD &
NBD_SERVER=$!
trap "kill $NBD_SERVER" EXIT  

echo DOCKER-RUN guestfish
docker run -v$(pwd)/$SYSROOT:/sysroot --rm -i --add-host=host.docker.internal:host-gateway bkahlert/libguestfs:edge guestfish << EOF
add '' protocol:nbd server:host.docker.internal
run
part-init /dev/sda mbr
part-add /dev/sda p $PART1_START $PART1_END
part-add /dev/sda p $PART2_START $PART2_END
mkfs ext2 /dev/sda1 blocksize:1024
mkfs ext2 /dev/sda2
mount /dev/sda1 /
copy-in /sysroot /
glob mv /sysroot/* /
rmdir /sysroot
EOF
