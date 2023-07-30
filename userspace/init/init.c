#include <kfs/kernel.h>

int main(void) {
	pid_t pid = fork();

	// child path
	if (pid == 0) {
		int result = exec("test_relation.bin");

		// exec failed.
		_exit(result);
	}

	pid_t child;
	int status;
	for (;;) {
		while ((child = waitpid(-1, &status, 0)) > 0) {
			fortytwo(child);
			fortytwo(status);
		}
	}
	return 0;
}
