SYSROOT=$1

if [ -z "$SYSROOT" ]; then
    echo "usage: make-sysroot.sh <sysroot>"
    exit 1
fi

DIRS="bin boot dev etc lib media mnt opt run sbin srv tmp usr var home root"
for DIR in $DIRS; do
    mkdir $SYSROOT/$DIR
done

ALPINE_SYSROOT="scripts/hdd/alpine-minirootfs-3.19.1-x86.tar.gz"
tar -xf $ALPINE_SYSROOT -C $SYSROOT/

USERS="cjeon"
for USER in $USERS; do
    mkdir $SYSROOT/home/$USER
done

mkdir $SYSROOT/lib/modules

cat << 'EOF' > $SYSROOT/etc/passwd
root::0:0::/root:/bin/sh
cjeon:$6$xKfGiVIDU2eFHpz9$CIrn5g9ODPQM1VznJ941RjEeoPvaKNHak1o7rrUJR1jXg/kZL7bmQcv5xD3GFLCn39dhWRlsMmbNam59tDIgh0:1000:1000::/home/cjeon:/bin/shell
EOF

cat << 'EOF' > $SYSROOT/root/.env
USER=root
HOME=/root
ABC=123
EOF

cat << 'EOF' > $SYSROOT/home/cjeon/.env
USER=cjeon
HOME=/home/cjeon
DEF=456
EOF

chmod -R 777 $SYSROOT # TODO remove this
