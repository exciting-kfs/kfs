SYSROOT=$1

if [ -z "$SYSROOT" ]; then
    echo "usage: make-sysroot.sh <sysroot>"
    exit 1
fi

DIRS="bin boot dev etc lib media mnt opt run sbin srv tmp usr var home root"
USERS="cjeon"

for DIR in $DIRS; do
    mkdir $SYSROOT/$DIR
done

for USER in $USERS; do
    mkdir $SYSROOT/home/$USER
done

mkdir $SYSROOT/lib/modules

cat << 'EOF' > $SYSROOT/etc/passwd
root::0:0::/root:shell.bin
cjeon:$6$xKfGiVIDU2eFHpz9$CIrn5g9ODPQM1VznJ941RjEeoPvaKNHak1o7rrUJR1jXg/kZL7bmQcv5xD3GFLCn39dhWRlsMmbNam59tDIgh0:1000:1000::/home/cjeon:test_file.bin
EOF
