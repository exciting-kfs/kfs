#include <kfs/kernel.h>
#include <kfs/ft.h>

void print_id(const char *s) {
	ft_putstr("\n");
	ft_putstr(s);
	ft_putstr("\npid: ");
	ft_putnbr(getpid());
	ft_putstr("\nppid: ");
	ft_putnbr(getppid());
	ft_putstr("\npgid: ");
	ft_putnbr(getpgrp());
	ft_putstr("\nsid: ");
	ft_putnbr(getsid());
	ft_putstr("\n");
}

void wait_newline() {
	char c = 0;

	ft_putstr("\nPRESS A NEW LINE TO CONTINUE.....");
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
	kill(-1, SIGINT);
	write(0, "****done****\n", 13);
}

void sig_int(int sig) {
	(void)sig;
}

void sig_quit(int sig) {
	(void)sig;
	ft_putstr("\nsig quit\n");
}

void child() {
	signal(SIGINT, SIG_DFL);
	signal(SIGQUIT, sig_quit);

	sched_yield();

	print_id("child");

	while (1) {
		sched_yield();
	}
}

int main(void) {
	int ret;
	pid_t pid;

	write(0, "test start!\n", 12);
	signal(SIGINT, sig_int);
	signal(SIGQUIT, SIG_IGN);
	print_id("initial state");
	next_test();

	// TEST 1: setsid()
	// - check that a new session is created.
	// - press `F2` to change terminal and see results.
	title(1);
	setsid();
	open();
	print_id("after setsid");
	next_test();

	// TEST 2: setsid()
	// - check that the session leader can not make a new session.
	title(2);
	ret = setsid();
	ft_putstr("\n");
	ft_putnbr(ret);
	ft_putstr("\n");
	next_test();

	// TEST 3: fork()
	// - check that the child process is in same process group.
	title(3);
	if (fork() == 0) {
		child();
	}
	next_test();

	// TEST 4: setpgid(0, 0)
	// - check that the child process has own `pgrp`.
	// - check `pid`, `pgid` allocation and deallcation.
	title(4);
	if (fork() == 0) {
		setpgid(0, 0);
		child();
	}
	print_id("parent");
	next_test();

	// TEST 5: setpgid(child, parent.pgrp)
	// - step 1: check that the child process has own `pgrp`(background).
	// - step 2: check that the child process is moved to parent's `pgrp`(foreground).
	title(5);
	print_id("parent");
	pid = fork();
	if (pid == 0) {
		setpgid(0, 0); // step 1: ignore `sig quit`.
		child();
	}
	wait_newline();
	setpgid(pid, getpgrp()); // step 2: receive `sig quit`.
	wait_newline();
	next_test();

	// TEST 6: setpgid(0, invalid)
	// - check errno::EPERM (-1).
	title(6);
	ret = setpgid(0, 42);
	ft_putstr("\n");
	ft_putnbr(ret);
	next_test();

	// TEST 7: setpgid(invalid, 0)
	// - check errno::ESRCH (-3).
	title(7);
	ret = setpgid(42, 0);
	ft_putstr("\n");
	ft_putnbr(ret);
	next_test();

	// TEST 8: session deallcation.
	title(8);
	return 0;
}
