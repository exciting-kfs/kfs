#include "kfs/ft.h"
#include <unistd.h>

__asm__  (".text\n"
	".global _start\n"
	"_start:\n"
	"	xor %ebp, %ebp\n"
	"	and $0xfffffff0, %esp\n"
	"	push $0\n"
	"	push %esi\n"
	"	push %edi\n"
	"	push %edx\n"
	"	push %esp\n"
	"	call _start_c\n"
);

int start();

void __libc_start_main(int (*main_fn)(int, char **, char **), int argc, char **argv, char **envp) {
	_exit(main_fn(argc, argv, envp));
}

void  _start_c(long *args) {
	__libc_start_main(start, args[0], (void *)args[1], (void *)args[2]);
}
