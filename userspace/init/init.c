#include <kfs/kernel.h>

int main(void) {
	int pid = fork();
	if (pid == 0) {
		exec("test_pipe.bin");
		_exit(1);
	}

	for (;;) {
		int status;
		pid_t pid = waitpid(-1, &status, 0);
	}

	return 0;
}
