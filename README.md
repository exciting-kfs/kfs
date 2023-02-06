# cargo만을 이용해서 커널 바이너리 성성

## 설정

file: $CARGO_HOME/config.toml
```toml
[unstable]
build-std-features= ["compiler-builtins-mem"]
build-std = ["core", "compiler_builtins"]

```

## 빌드
```shell
cargo build --target ./target.json
```

## 결과

```
Symbol table '.symtab' contains 4 entries:
   Num:    Value  Size Type    Bind   Vis      Ndx Name
     0: 00000000     0 NOTYPE  LOCAL  DEFAULT  UND 
     1: 00000000     0 FILE    LOCAL  DEFAULT  ABS 4uwhhi6mzx6h9uy6
     2: 00100000    24 OBJECT  LOCAL  DEFAULT    1 _ZN4kfs17_HEADER[...]
     3: 00100020     4 FUNC    GLOBAL DEFAULT    2 kernel_entry
```