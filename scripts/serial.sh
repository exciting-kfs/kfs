#! /bin/bash

if [ $# -lt 2 ]; then
	echo 'Usage: serial.sh "kernbuf serial" "unit_test serial"'; exit 1
fi

SERIAL="$1"
UNIT_TEST="$2"

if [ -p $SERIAL ]; then
    rm -f $SERIAL
fi

if [ -p $UNIT_TEST ]; then
    rm -f $UNIT_TEST
fi

mkfifo $SERIAL
mkfifo $UNIT_TEST