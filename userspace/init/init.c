#include <fcntl.h>
#include <signal.h>
#include <unistd.h>

#include <sys/wait.h>

#include "kfs/ft.h"

int start(void) {
	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);
	

	int pid = fork();
	if (pid == 0) {
			signal(SIGINT, SIG_IGN);
			signal(SIGQUIT, SIG_IGN);
			execve("getty.bin", NULL, NULL);
			_exit(128);
	}

	for (;;) {
		int status;
		waitpid(-1, &status, 0);
	}

	return 0;
}
