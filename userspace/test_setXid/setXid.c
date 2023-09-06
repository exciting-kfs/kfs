#include <fcntl.h>
#include <signal.h>
#include <unistd.h>

#include "kfs/ft.h"
#include "kfs/kernel.h"

void print_id(const char *s) {
	ft_putstr("\n");
	ft_putstr(s);
	ft_putstr("\npid:  ");
	ft_putnbr(getpid());
	ft_putstr("\nppid: ");
	ft_putnbr(getppid());
	ft_putstr("\npgid: ");
	ft_putnbr(getpgrp());
	ft_putstr("\nsid:  ");
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

void next_test() {
	wait_newline();
	kill(-1, SIGINT);
	write(0, "****done****\n", 13);
}

void title(int num, const char *s) {
	ft_putstr("\n TEST");
	ft_putnbr(num);
	ft_putstr(": ");
	ft_putstr(s);
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

	signal(SIGINT, sig_int);
	signal(SIGQUIT, SIG_IGN);
	print_id("initial state");
	wait_newline();

	title(1, "setsid()\n");
	ft_putstr("- check that a new session is created.\n");
	ft_putstr("- press `F2` to change terminal and see results.\n");
	setsid();
	close(0);
	close(1);
	close(2);
	print_id("after setsid");
	next_test();

	title(2, "setsid()\n");
	ft_putstr("- check that the session leader can not make a new session.\n");
	ret = setsid();
	ft_putstr("\n");
	ft_putnbr(ret);
	ft_putstr("\n");
	next_test();

	title(3, "fork()\n");
	ft_putstr("- check that the child process is in same process group.\n");
	if (fork() == 0) {
		child();
	}
	next_test();

	title(4, "setpgid(0,0)\n");
	ft_putstr("- check that the child process has own `pgrp`.\n");
	ft_putstr("- check `pid`, `pgid` allocation and deallcation.\n");
	if (fork() == 0) {
		setpgid(0, 0);
		child();
	}
	print_id("parent");
	next_test();

	title(5, "setpgid(child, parent pgrp)\n");
	print_id("parent");
	ft_putstr("- step 1: check that the child process has own `pgrp`(background).\n");
	ft_putstr("  - todo: check that the child doesn't receive `sig quit`");
	pid = fork();
	if (pid == 0) {
		setpgid(0, 0);
		child();
	}
	wait_newline();
	setpgid(pid, getpgrp());
	ft_putstr(
	    "- step 2: check that the child process is moved to parent's `pgrp`(foreground).\n");
	ft_putstr("  - todo: check that the child receive `sig quit`.");
	wait_newline();
	next_test();

	title(6, "setpgid(0, invalid)\n");
	ft_putstr("- check errno::EPERM (-1).\n");
	ret = setpgid(0, 42);
	ft_putstr("\n");
	ft_putnbr(ret);
	next_test();

	title(7, "setpgid(invalid, 0)\n");
	ft_putstr("- check errno::ESRCH (-3)\n");
	ret = setpgid(42, 0);
	ft_putstr("\n");
	ft_putnbr(ret);
	next_test();

	title(8, "session deallocation.\n");
	ft_putstr("- check printk result.\n");
	ft_putstr("- press `F1` to return.\n");
	return 0;
}
