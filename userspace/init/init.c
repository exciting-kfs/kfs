#include <kfs/ft.h>
#include <kfs/kernel.h>

const char *tests[] = {
    "test_pipe.bin", "test_setXid.bin", "test_sig.bin", "test_sig_stop_cont.bin", NULL,
};

void waitpid_verbose(pid_t pid) {
	int status;
	pid_t real_pid;

	real_pid = waitpid(pid, &status, 0);
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
}

int main(void) {

	for (const char **p = tests; *p; p++) {
		int pid = fork();
		if (pid == 0) {
			ft_putstr("run ");
			ft_putstr(*p);
			ft_putstr("\n");
			exec(*p);
			_exit(128);
		}
		waitpid_verbose(pid);
	}

	ft_putstr("====TEST FINISHED.====\n");

	for (;;) {
		waitpid_verbose(-1);
	}

	return 0;
}
