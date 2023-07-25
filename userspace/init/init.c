#include <kfs/kernel.h>

int main(void) {
	for (;;) {
		pid_t pid = fork();

		// child path
		if (pid == 0) {
			int result = exec("fortytwo.bin");

			// exec failed. report error number and return.
			fortytwo(result);
			break;
		}
	}
	return 0;
}
