# x86_32 Kernel project

## Dependencies

---

### Kernel binary

- rust (nightly)
- nasm 
- GNU binutils
- make

---

### Run & Debug

- grub2
- xorriso
- qemu
- lldb

---

## Makefile Configs

- `RELEASE_MODE`

	- `y`: Enable all optimizations.
	- `n`: Debug mode. Generate debug symbols and enable run-time checkings.

- `DEBUG_WITH_VSCODE`

	- `y`: Debug with vscode. [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) is needed.
	- `n`: Debug with plain LLDB.

---

## Makefile Targets

### Build

- `all` (default target), `rescue`: Create ISO rescue image.

- `build`: Create kernel binary.

- `doc`: Generate `libkernel.a` document.

- `clean`: Delete build artifacts.

- `re`: Perform clean-build

### Run

- `run`: Run kernel with Qemu.

- `debug`: Debug kernel with LLDB.

- `dmesg`: Read kernel message buffer.

### Utils

- `dump-header`: Dump kernel ELF headers.

- `dump-text`: Dump kernel .text sections.

- `size`: Show size of kernel binary.

- `doc-open`: Open `libkernel.a` document.
