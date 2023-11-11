#include <fcntl.h>
#include <signal.h>
#include <unistd.h>

#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/wait.h>

#include "kfs/ft.h"
#include "kfs/kernel.h"
#include "kfs/libft.h"

int main(void) {
	mkdir("/dev", 0777);
	mount("dev", "/dev", "devfs");

	mkdir("/proc", 0777);
	mount("proc", "/proc", "procfs");

	mkdir("/sys", 0777);
	mount("sysfs", "/sys", "sysfs");

	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);

	int ret = init_module("/lib/modules/kbd.ko");
	ft_printf("insmod kbd.ko: %d\n", ret);

	int pid = fork();
	if (pid == 0) {
		signal(SIGINT, SIG_IGN);
		signal(SIGQUIT, SIG_IGN);
		char *argv[] = {"getty", NULL};
		char *envp[] = {NULL};
		execve("/bin/getty", argv, envp);
		_exit(128);
	}

	for (;;) {
		int status;
		waitpid(-1, &status, 0);
	}

	return 0;
}
