#include <unistd.h>

#include "kfs/kernel.h"
#include "kfs/libft.h"

int main(void) {	
	char *argv[] = {
		"v1",
		"v2",
		"v3",
		NULL,
	};

	char *envp[] = {
		"e1",
		"e2",
		"e3",
		NULL,
	};


	int ret = execve("test_argv.bin", argv, envp);
	ft_printf("error: %d\n", ret);

	return 0;
}