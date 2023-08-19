#!/bin/bash

HDD="/tmp/disk.img"
MB=1048576
SIZE=+20M; tmp=${SIZE//+}; tmp=${tmp//[^0-9]}; NUM=$tmp
NEXT_MB=$(($NUM * $MB + $MB))

source /util.sh

function install_ext2() {
    local free=$(losetup -f)

    losetup $free $HDD -o $1
    mkfs -t ext2 $free
    losetup -d $free
}

function main () {
    # make partition
    silent fdisk $HDD <<EOF
n
p
1

$SIZE
n
p
2


w
EOF

    silent install_ext2 $MB
    silent install_ext2 $NEXT_MB
}

main $@
# fdisk -l $HDD
# exec /bin/bash




