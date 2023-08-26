#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include "kfs/ft.h"
#include "kfs/kernel.h"

void sig_int(int num) {
	(void)num;
	char *s = "hello\n";
	write(0, s, 6);
}

void sig_int_block(int sig) {
	(void)sig;

	char buf[3] = {
	    0,
	};
	ft_putstr("blocked by read syscall.\n");
	int ret = read(0, buf, 3);
	ft_putstr("read size: ");
	ft_putnbr(ret);
	ft_putstr("\n`SIGINT handler blocked by syscall` done\n");
}

void sig_quit(int num) {
	(void)num;
	write(0, "sig quit!\n", 11);
}

void sig_action(int num, siginfo_t *info, void *sig_ctx) {
	(void)num;
	(void)sig_ctx;

	ft_putstr("sig action!\n");

	ft_putstr("==== sig info ====\n");

	ft_putstr("num: ");
	ft_putnbr_x(info->num);

	ft_putstr("  pid: "); // TODO '\t' not working.
	ft_putnbr_x(info->pid);

	ft_putstr("\nuid: ");
	ft_putnbr_x(info->uid);

	ft_putstr("  code: ");
	ft_putnbr_x(info->code);

	ucontext_t *u = sig_ctx;
	ft_putstr("\n==== sig context ====");

	ft_putstr("\nebp: ");
	ft_putnbr_x(u->ebp);
	ft_putstr("  edi: ");
	ft_putnbr_x(u->edi);
	ft_putstr("\nesi: ");
	ft_putnbr_x(u->esi);
	ft_putstr("  edx: ");
	ft_putnbr_x(u->edx);
	ft_putstr("\necx: ");
	ft_putnbr_x(u->ecx);
	ft_putstr("  ebx: ");
	ft_putnbr_x(u->ebx);
	ft_putstr("\neax: ");
	ft_putnbr_x(u->eax);
	ft_putstr("  ds: ");
	ft_putnbr_x(u->ds);
	ft_putstr("\nes: ");
	ft_putnbr_x(u->es);
	ft_putstr("  fs: ");
	ft_putnbr_x(u->fs);
	ft_putstr("\ngs: ");
	ft_putnbr_x(u->gs);
	ft_putstr("  handler: ");
	ft_putnbr_x(u->handler);
	ft_putstr("\nerror_code: ");
	ft_putnbr_x(u->error_code);
	ft_putstr("  eip: ");
	ft_putnbr_x(u->eip);
	ft_putstr("\ncs: ");
	ft_putnbr_x(u->cs);
	ft_putstr("  eflags: ");
	ft_putnbr_x(u->eflags);
	ft_putstr("\nesp: ");
	ft_putnbr_x(u->esp);
	ft_putstr("  ss: ");
	ft_putnbr_x(u->ss);
	ft_putstr("\nmask: ");
	ft_putnbr_x(u->mask);
	ft_putstr("  syscall_ret: ");
	ft_putnbr(u->syscall_ret);

	write(0, "\nsig action done\n", 17);
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

int main(void) {
	pid_t pid;
	int stat;

	title(1, "signal handler\n");
	ft_putstr("- check that signal handler is working.\n");
	ft_putstr("- check returning to the interrupted position properly from signal handler "
		  "after processing signal.\n");
	ft_putstr("- send SIGINT, SIGQUIT using keyboard.");
	signal(SIGINT, sig_int);
	signal(SIGQUIT, sig_quit);
	next_test();

	title(2, "syscall: signal: SIG_DFL, SIG_IGN\n");
	ft_putstr("- check that DFL and IGN features is working.\n");
	ft_putstr("- SIGINT: DFL, SIGQUIT: IGN\n");
	signal(SIGINT, SIG_IGN);
	signal(SIGQUIT, SIG_IGN);
	pid = fork();
	if (pid == 0) {
		signal(SIGINT, SIG_DFL);
		while (1) {
			sched_yield();
		}
	}
	ft_putstr("- MUST send SIGINT to child.");
	waitpid(pid, &stat, 0);
	next_test();

	title(3, "syscall: signal: deferred\n");
	ft_putstr("- check that the signal in itself handler is deferred\n");
	ft_putstr("- check returning to the interrupted position properly from signal handler "
		  "after processing signal.\n");
	ft_putstr("- send SIGINT, SIGQUIT using keyboard.");
	signal(SIGINT, sig_int_block);
	signal(SIGQUIT, sig_quit);
	next_test();

	title(4, "syscall: signal: return value\n");
	if (signal(SIGINT, SIG_DFL) != sig_int_block)
		return 1;
	if (signal(SIGINT, SIG_IGN) != SIG_DFL)
		return 1;
	if (signal(SIGINT, sig_int) != SIG_IGN)
		return 1;
	next_test();

	title(5, "syscall: sigaction: act or old is null.\n");
	struct sigaction a = {.sa_handler = sig_quit, .sa_mask = 0};
	struct sigaction b = {.sa_handler = 0, .sa_mask = 0};
	sigaction(SIGQUIT, &a, NULL);
	sigaction(SIGQUIT, NULL, &b);
	if (b.sa_handler != sig_quit) {
		ft_putstr("invalid old\n");
		return 1;
	}
	next_test();

	title(6, "syscall: sigaction: mask\n");
	ft_putstr("- send SIGINT, SIGQUIT using keyboard.\n");
	ft_putstr("- expectation: in SIGINT handler, SIGQUIT is blocked by mask.\n");
	struct sigaction c = {.sa_handler = sig_int_block, .sa_mask = sigmask(SIGQUIT)};
	sigaction(SIGINT, &c, NULL);
	next_test();

	title(7, "syscall: sigaction: SA_NODEFER.(SIGINT)\n");
	ft_putstr("- sinario: type SIGINT more than twice. After that, type enter.\n");
	struct sigaction d = {
	    .sa_handler = sig_int_block, .sa_mask = sigmask(SIGQUIT), .sa_flags = SA_NODEFER};
	sigaction(SIGINT, &d, NULL);
	next_test();

	title(8, "syscall: sigaction: SA_RESTART.(SIGINT)\n");
	ft_putstr("- sinario: type SIGINT more than twice. After that, type enter twice.\n");
	struct sigaction e = {.sa_handler = sig_int_block,
			      .sa_mask = sigmask(SIGQUIT),
			      .sa_flags = SA_NODEFER | SA_RESTART};
	sigaction(SIGINT, &e, NULL);
	next_test();

	title(9, "syscall: sigaction: SA_RESETHAND\n");
	ft_putstr("- SIGINT: SA_RESETHAND, SIGQUIT: IGN\n");
	struct sigaction f = {.sa_handler = sig_int_block, .sa_mask = 0, .sa_flags = SA_RESETHAND};
	signal(SIGINT, SIG_IGN);
	signal(SIGQUIT, SIG_IGN);
	pid = fork();
	if (pid == 0) {
		sigaction(SIGINT, &f, NULL);
		while (1) {
			sched_yield();
		}
	}
	ft_putstr("- MUST send SIGINT to child.");
	waitpid(pid, &stat, 0);
	next_test();

	title(10, "syscall: sigaction: SA_SIGINFO.(SIGINT)\n");
	ft_putstr("- check that the params of the signal handler is correct.\n");
	struct sigaction g = {.sa_sigaction = sig_action, .sa_mask = 0, .sa_flags = SA_SIGINFO};
	sigaction(SIGINT, &g, NULL);
	next_test();

	return 0;
}
