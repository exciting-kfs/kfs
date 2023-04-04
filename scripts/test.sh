RESCUE="$1"
shift

SERIAL="$1"
shift

UNIT_TEST="$1"
shift

until [ -p $UNIT_TEST ] && [ -p $SERIAL ]
do
    sleep 1
done

trap "rm -f $SERIAL $UNIT_TEST" EXIT
if [ -z $UNIT_TEST_FUNC ]; then
    echo "Error: test.sh: MUST set 'UNIT_TEST_FUNC' env before running test."
    exit
fi

echo " unit_test $UNIT_TEST_FUNC" >> $UNIT_TEST & # why lost 1st character...?

# -m 3968(4096 - 128): almost maximum memory in x86 (without PAE)
qemu-system-i386                    \
    -machine pc,max-ram-below-4g=4G \
    -m 3968                         \
    -boot d                         \
    -vga std                        \
    -cdrom $RESCUE                  \
    -serial pipe:$SERIAL            \
    -serial pipe:$UNIT_TEST         \
    $@				                &

QEMU_PID=$!

trap "rm -f $SERIAL $UNIT_TEST && kill $!" EXIT

cat $SERIAL
