#include <kfs/kernel.h>

// we don't have .BSS yet :(

int main(void) {

	int pipe_pair[2];
	int ret = pipe(pipe_pair);
	if (ret)
		return 1;

	int pid = fork();

	if (pid == 0) {
		char c[] = "A\n";
		for (;;)
			write(pipe_pair[1], c, 2);
	}

	char buf[32];

	for (;;) {
		read(pipe_pair[0], buf, 2);
		write(1, buf, 2);
	}

	return 0;
}
