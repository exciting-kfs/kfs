#include "kfs/ft.h"
#include "kfs/libft.h"
#include <unistd.h>

__asm__  (".section .entry\n"
	".global _start\n"
	"_start:\n"
	"	push %esp\n"
	"	call _start_c\n"
);


void __libc_start_main(int (*main_fn)(int, char **, char **), int argc, char **argv, char **envp) {
	_exit(main_fn(argc, argv, envp));
}

int main();

void  _start_c(long *args) {
	int argc = args[0];
	char **argv = (void *)(&args[1]);
	char **envp = (void *)(&argv[argc + 1]);
	__libc_start_main(main, argc, argv, envp);
}
