#include "kfs/ft.h"
#include <unistd.h>

__asm__  (".section .entry\n"
	".global _start\n"
	"_start:\n"
	"	push %esi\n"
	"	push %edi\n"
	"	push %edx\n"
	"	push %esp\n"
	"	call _start_c\n"
);


void __libc_start_main(int (*main_fn)(int, char **, char **), int argc, char **argv, char **envp) {
	_exit(main_fn(argc, argv, envp));
}

int main();

void  _start_c(long *args) {
	__libc_start_main(main, args[0], (void *)args[1], (void *)args[2]);
}
