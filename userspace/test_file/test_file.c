#include <unistd.h>

#include "kfs/kernel.h"
#include "kfs/libft.h"


int main(void) {	

	char *argv[] = {
		"/bin/args",
		"1",
		"2",
		"3",
		NULL,
	};

	char *envp[] = {
		"A=b",
		"B=c",
		"C=d",
		NULL,
	};

	int ret = execve("/bin/args", argv, envp);
	ft_printf("failed to execve (ret = %d)\n", ret);

	return 0;
}