# cargo만을 이용해서 커널 바이너리 성성

## 설정

### enable rust nightly
```
$ rustup default nightly
```

## 빌드
```shell
$ cargo build
```

## 결과

```shell
$ objdump -x ./target/i686-unknown-none-elf/debug/kernel
```
```
SYMBOL TABLE:
00000000 l    df *ABS*  00000000 388nr5smw4oaeq89
00100000 l     O .boot  00000018 _ZN6kernel17_MULTIBOOT_HEADER17hb16eff14b948178bE
00100020 g     F .text  00000004 kernel_entry
```