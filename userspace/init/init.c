#include <kfs/kernel.h>

// typedef void (a)(void);

int main(void) {
	int a = 123;
	// child path
	pid_t pid = fork();
	if (pid == 0) {
		((void (*)(void))(&a))();
		// fortytwo(*p);
		_exit(0);
	}

	pid_t child;
	int status;
	for (;;) {
		while ((child = waitpid(-1, &status, 0)) > 0) {
			if (WIFSIGNALED(status)) {
				fortytwo(WTERMSIG(status));
				fortytwo(child);
			}
		}
	}
	return 0;
}
