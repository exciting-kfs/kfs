#include <fcntl.h>
#include <unistd.h>

#include <sys/socket.h>
#include <sys/stat.h>

#include "kfs/ft.h"
#include "kfs/kernel.h"

struct sockaddr_un {
	unsigned short family;
	char path[108];
};

int do_child(void) {
	int sock = socket(PF_LOCAL, SOCK_DGRAM, 0);

	struct sockaddr_un addr = {
	    .family = PF_LOCAL,
	    .path = "/test.sock",
	};

	int ret = connect(sock, (void *)&addr, sizeof(addr));

	if (ret < 0) {
		ft_putstr("bind: ");
		ft_putnbr(ret);
		_exit(1);
	}

	for (int i = 0; i < 10; i++) {
		int ret = write(sock, "123456678\n", 10);

		if (ret < 0) {
			ft_putstr("child: wirte");
			ft_putnbr(ret);
			_exit(1);
		}
	}

	return 0;
}

int main(void) {

	int sock = socket(PF_LOCAL, SOCK_DGRAM, 0);

	struct sockaddr_un addr = {
	    .family = PF_LOCAL,
	    .path = "/test.sock",
	};

	int ret = bind(sock, (void *)&addr, sizeof(addr));

	if (ret < 0) {
		ft_putstr("bind: ");
		ft_putnbr(ret);
		_exit(1);
	}

	int pid = fork();
	if (pid == 0) {
		_exit(do_child());
	}

	for (;;) {
		char buf[128];
		int ret = read(sock, buf, 128);
		if (ret < 0) {
			ft_putstr("main: read");
			ft_putnbr(ret);
			_exit(1);
		}
		write(1, buf, ret);
	}

	return 0;
}