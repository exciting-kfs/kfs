#include <kfs/kernel.h>

const char *base_10 = "0123456789";
const char *base_16 = "0123456789abcdef";

size_t ft_strlen(const char *s) {
	size_t i = 0;
	while (s[i]) {
		i++;
	}
	return i;
}

void ft_putstr_fd(int fd, const char *s) {
	size_t n = ft_strlen(s);
	write(fd, s, n);
}

void sig_int(int num) {
	(void)num;
	write(0, "sig int!\n", 9);
}

void wait_newline() {
	char c = 0;

	ft_putstr_fd(0, "\nPRESS A NEW LINE TO CONTINUE.....");
	while (c != '\n') {
		read(0, &c, 1);
	}
}

void title(int num) {
	write(0, "\ntest", 5);
	ft_putnbr(num);
}

void next_test() {
	wait_newline();
	write(0, "****done****\n", 13);
}

void child() {
	unsigned int cnt = 0;
	char c = 'c';
	signal(SIGINT, sig_int);

	while (1) {
		if (cnt % 1000 == 0) {
			write(0, &c, 1);
		}
		sched_yield();
		cnt++;
	}
}

int main(void) {
	pid_t pid = fork();

	if (pid == 0) {
		child();
	}
	sched_yield();

	// TEST 1: SIGSTOP
	// - check that child process is stopped. (it doesn't print 'c')
	title(1);
	kill(pid, SIGSTOP);
	wait_newline();

	// TEST 2: SIGCONT
	// - check that child process is running. (it prints 'c')
	title(2);
	kill(pid, SIGCONT);
	wait_newline();

	// TEST 3: DeepSleep
	// - step1: check that child process is stopped. (it doesn't print 'c')
	// - step2: check that child process is running. (it prints 'c')
	// - step3: check that child process receives and does SIGINT signal twice.
	title(3);
	kill(pid, SIGSTOP);
	kill(pid, SIGINT);
	kill(pid, SIGINT);
	wait_newline();
	kill(pid, SIGCONT);
	wait_newline();

	return 0;
}
