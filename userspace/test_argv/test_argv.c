#include <fcntl.h>
#include <unistd.h>

#include "kfs/ft.h"
#include "kfs/libft.h"

int start(int argc, char **argv, char **envp) {

	ft_printf("argc: %d\n", argc);

	for (char **p = argv; *p; ++p) {
		ft_printf("ARGV: %s\n", *p);
	}

	for (char **p = envp; *p; ++p) {
		ft_printf("ENVP: %s\n", *p);
	}

	return 0;
}