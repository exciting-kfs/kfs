#include <kfs/kernel.h>

const char *test = "abcd";

int main(void) {
	for (;;) {
		pid_t cpid = fork();
		if (cpid != 0) {
			fortytwo(cpid);
			break;
		}
	}
	return test[0];
}
