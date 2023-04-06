RESCUE="$1"
shift

COM1="$1"
shift

COM2="$1"
shift

until [ -p $COM2 ] && [ -p $COM1 ]
do
    sleep 1
done

trap "rm -f $COM1 $COM2" EXIT
if [ -z $UNIT_TEST ]; then
    echo "Error: test.sh: MUST set 'UNIT_TEST' env before running test."
    exit
fi

echo " unit_test $UNIT_TEST" >> $COM2 & # why this command lost 1st character...?

# -m 3968(4096 - 128): almost maximum memory in x86 (without PAE)
qemu-system-i386                    \
    -machine pc,max-ram-below-4g=4G \
    -m 3968                         \
    -boot d                         \
    -vga std                        \
    -cdrom $RESCUE                  \
    -serial pipe:$COM1              \
    -serial pipe:$COM2              \
    -display none                   \
    $@				                &

QEMU_PID=$!

trap "rm -f $COM1 $COM2 && kill $!" EXIT

cat $COM1
