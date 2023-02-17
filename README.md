# x86 32 bit kernel project

## Dependencies

### build kernel binary
- cargo / rustc (nightly)
- nasm 

### build rescue image
- grub2 (compiled with CC_TARGET=i686-elf-gcc)
- xorriso

### run with qemu
- qemu

### debug
- lldb

## Configure

### enable rust nightly
```
$ rustup default nightly
```

## Build & Run

### create kernel binary
```shell
$ cargo build

OR

$ make build
```

### create rescue image
```shell
$ make rescue
```

### run rescue image with qemu
```shell
$ make run
```

### debug
```shell
$ make debug
```