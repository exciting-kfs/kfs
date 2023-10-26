#include <fcntl.h>
#include <signal.h>
#include <unistd.h>

#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/wait.h>

#include "kfs/ft.h"
#include "kfs/libft.h"
#include "kfs/kernel.h"

int main(void) {
	mkdir("/dev", 0777);
	mount("dev", "/dev", "devfs");

	mkdir("/proc", 0777);
	mount("proc", "/proc", "procfs");

	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);

	int ret = init_module("/lib/modules/kbd.ko");
	ft_printf("insmod kbd.ko: %d\n", ret);

	int pid = fork();
	if (pid == 0) {
		signal(SIGINT, SIG_IGN);
		signal(SIGQUIT, SIG_IGN);
		execve("/bin/getty", NULL, NULL);
		_exit(128);
	}

	for (;;) {
		int status;
		waitpid(-1, &status, 0);
	}

	return 0;
}
