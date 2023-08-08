#include <kfs/ft.h>
#include <kfs/kernel.h>

void sig_int(int num) {
	(void)num;
	write(0, "sig int!\n", 9);
}

void wait_newline() {
	char c = 0;

	ft_putstr("\nPRESS A NEW LINE TO CONTINUE.....");
	while (c != '\n') {
		read(0, &c, 1);
	}
}

void next_test() {
	wait_newline();
	write(0, "****done****\n", 13);
}

void title(int num, const char *s) {
	ft_putstr("\n TEST");
	ft_putnbr(num);
	ft_putstr(": ");
	ft_putstr(s);
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

	title(1, "SIGSTOP\n");
	ft_putstr("- check that child process is stopped. (it doesn't print 'c')\n");
	kill(pid, SIGSTOP);
	wait_newline();

	title(2, "SIGCONT\n");
	ft_putstr("- check that child process is running. (it prints 'c')\n");
	kill(pid, SIGCONT);
	wait_newline();

	title(3, "DeelSleep\n");
	ft_putstr("- step1: check that child process is stopped. (it doesn't print 'c')\n");
	kill(pid, SIGSTOP);
	kill(pid, SIGINT);
	kill(pid, SIGINT);
	wait_newline();
	ft_putstr("- step2: check that child process is running. (it prints 'c')\n");
	ft_putstr("- step3: check that child process receives and does SIGINT signal twice.\n");
	kill(pid, SIGCONT);
	wait_newline();
	kill(pid, SIGKILL);

	return 0;
}
