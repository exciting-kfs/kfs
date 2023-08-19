from debian:latest

run apt update && apt install -y fdisk grub less

copy ./entry.sh /entry.sh
copy ./util.sh /util.sh

entrypoint ["/entry.sh"]