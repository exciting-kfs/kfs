#!/bin/bash

IMAGE_NAME=kfs-build-hdd

source $(dirname "$0")/util.sh

if ! silent docker images | grep $IMAGE_NAME; then
    silent docker build -t $IMAGE_NAME ./hdd
fi

silent dd if=/dev/zero of=/tmp/disk.img bs=512 count=81920  # 40MB
docker run -v /tmp:/tmp -it --privileged --cap-add=CAP_MKNOD --cap-add=SYS_ADMIN --device-cgroup-rule="b 7:* rmw" $IMAGE_NAME /bin/sh
