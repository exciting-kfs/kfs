#include <kfs/kernel.h>

int main(void) {
	for (;;) {
		pid_t pid = fork();

		if (pid == 0) {
			fortytwo(1);
			break;
		}

		waitpid(pid, NULL, 0);
	}
	return 0;
}
