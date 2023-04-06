#! /bin/bash

if [ $# -lt 2 ]; then
	echo 'Usage: serial.sh "kernbuf serial" "unit_test serial"'; exit 1
fi

COM1="$1"
COM2="$2"

if [ -p $COM1 ]; then
    rm -f $COM1
fi

if [ -p $COM2 ]; then
    rm -f $COM2
fi

mkfifo $COM1
mkfifo $COM2