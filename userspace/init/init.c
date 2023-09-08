#include <sys/wait.h>
#include <unistd.h>

#include "fcntl.h"
#include "kfs/ft.h"

const char *tests[] = {
    "shell.bin",    "test_file.bin", "test_sig_stop_cont.bin",
    "test_sig.bin", "test_pipe.bin", "test_setXid.bin",
    NULL,
};

void waitpid_verbose(pid_t pid, const char *test_name) {
	int status;
	pid_t real_pid;

	real_pid = waitpid(pid, &status, 0);
	ft_putstr("\n");
	if (real_pid < 0) {
		ft_putstr("init: waitpid: err=");
		ft_putnbr(real_pid);
		ft_putstr("\n");
	} else {
		ft_putstr("init: waitpid: pid=");
		ft_putnbr(real_pid);
		if (WIFEXITED(status)) {
			ft_putstr(" exit=");
			ft_putnbr(WEXITSTATUS(status));
		} else if (WIFSIGNALED(status)) {
			ft_putstr(" signal=");
			ft_putnbr(WTERMSIG(status));
		}
		ft_putstr("\n");
	}
	ft_putstr("DONE: ");
	ft_putstr(test_name);
	ft_putstr("\n\n");
}

int main(void) {
	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);
	open("/dev/tty1", O_RDWR);

	for (const char **p = tests; *p; p++) {
		int pid = fork();
		if (pid == 0) {
			ft_putstr("\x1b[32mRUN: ");
			ft_putstr(*p);
			ft_putstr("\x1b[39m\n");
			exec(*p);
			_exit(128);
		}
		waitpid_verbose(pid, *p);
	}

	ft_putstr("====TEST FINISHED.====\n");

	for (;;) {
		waitpid_verbose(-1, "");
	}

	return 0;
}
