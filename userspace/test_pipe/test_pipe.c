#include <unistd.h>

const char buffer[] = "hello, world!\n";

void write_and_exit(int pipe_fds[]) {
	close(pipe_fds[0]);
	write(pipe_fds[1], buffer, sizeof(buffer) - 1);

	_exit(0);
}

void read_and_exit(int pipe_fds[]) {
	char buf[4096];
	close(pipe_fds[1]);

	ssize_t ret;
	ret = read(pipe_fds[0], buf, 4096);
	if (ret < 0) {
		_exit(3);
	}

	write(1, buf, ret);
	_exit(0);
}

void test_eof() {
	int pipe_fds[2];

	if (pipe(pipe_fds) < 0) {
		_exit(1);
	}

	pid_t pid;
	if ((pid = fork()) < 0) {
		_exit(2);
	}

	if (pid == 0) {
		write_and_exit(pipe_fds);
	} else {
		read_and_exit(pipe_fds);
	}
}

void test_sigpipe() {
	int pipe_fds[2];

	if (pipe(pipe_fds) < 0) {
		_exit(1);
	}
	close(pipe_fds[0]);

	char buf;
	write(pipe_fds[1], &buf, 1);

	_exit(0);
}

int main(void) {
	// test_eof();
	test_sigpipe();
	return 0;
}
