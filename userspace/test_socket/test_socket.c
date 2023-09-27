#include <unistd.h>

#include <sys/socket.h>
#include <sys/wait.h>

#include "kfs/internal/prelude.h"
#include "kfs/kernel.h"
#include "kfs/libft.h"

#define STRICT(expr)                                                                               \
	do {                                                                                       \
		int __ret = (expr);                                                                \
		if (__ret < 0) {                                                                   \
			ft_printf("%s:%d: [%s]: return was: %d\n", __FILE__, __LINE__, #expr,      \
				  __ret);                                                          \
			_exit(1);                                                                  \
		}                                                                                  \
	} while (0)

typedef struct _Barrier {
	int raw[2];
} Barrier;

void barrier_init(Barrier *barrier) {
	STRICT(pipe(barrier->raw));
}

void barrier_destroy(Barrier *barrier) {
	close(barrier->raw[0]);
	close(barrier->raw[1]);
}

void barrier_wait(Barrier *barrier) {
	char c;

	close(barrier->raw[1]);
	STRICT(read(barrier->raw[0], &c, 1));
	close(barrier->raw[0]);
}

void barrier_release(Barrier *barrier) {
	close(barrier->raw[0]);
	STRICT(write(barrier->raw[1], &"", 1));
	close(barrier->raw[1]);
}

struct sockaddr_un {
	unsigned short family;
	char data[108];
};

#define DEFINE_UNIX_SOCKADDR(name, path)                                                           \
	struct sockaddr_un name = {.family = PF_LOCAL, .data = path}

int dgram_basic_io_client(Barrier *barrier) {
	int sock;

	STRICT(sock = socket(PF_LOCAL, SOCK_DGRAM, 0));

	DEFINE_UNIX_SOCKADDR(addr, "/sock1.sock");
	barrier_wait(barrier);

	STRICT(connect(sock, (void *)&addr, sizeof(addr)));
	ssize_t nwrite;
	const char message[] = "hello!!!\n";

	STRICT(nwrite = write(sock, message, sizeof(message)));

	return 0;
}

int dgram_basic_io_server(Barrier *barrier) {
	int sock;

	STRICT(sock = socket(PF_LOCAL, SOCK_DGRAM, 0));

	DEFINE_UNIX_SOCKADDR(addr, "/sock1.sock");
	STRICT(bind(sock, (void *)&addr, sizeof(addr)));
	barrier_release(barrier);

	char buf[1024];
	ssize_t nread;
	STRICT(nread = read(sock, &buf, 1024));
	ft_printf("%s", buf);

	return 0;
}

int stream_basic_io_server(Barrier *barrier) {
	int sock;

	STRICT(sock = socket(PF_LOCAL, SOCK_STREAM, 0));

	DEFINE_UNIX_SOCKADDR(addr, "/sock2.sock");
	STRICT(bind(sock, (void *)&addr, sizeof(addr)));
	STRICT(listen(sock, 128));
	barrier_release(barrier);

	int client;
	STRICT(client = accept(sock, NULL, 0));

	char message[] = "hello from server\n";
	ssize_t nwrite;
	STRICT(nwrite = write(client, message, sizeof(message)));

	char buf[1024];
	ssize_t nread;
	STRICT(nread = read(client, buf, 19));
	ft_printf("%s", buf);

	return 0;
}

int stream_basic_io_client(Barrier *barrier) {
	int sock;

	STRICT(sock = socket(PF_LOCAL, SOCK_STREAM, 0));
	DEFINE_UNIX_SOCKADDR(addr, "/sock2.sock");

	barrier_wait(barrier);
	STRICT(connect(sock, (void *)&addr, sizeof(addr)));

	char buf[1024];
	ssize_t nread;
	STRICT(nread = read(sock, buf, 19));
	ft_printf("%s", buf);

	char message[] = "hello from client\n";
	STRICT(write(sock, message, sizeof(message)));

	return 0;
}

int dgram_send_recv_client(Barrier *barrier) {
	int sock;

	STRICT(sock = socket(PF_LOCAL, SOCK_DGRAM, 0));
	DEFINE_UNIX_SOCKADDR(addr, "/sock3.sock");

	barrier_wait(barrier);

	char message[] = "hello from client\n";
	ssize_t nwrite;
	STRICT(nwrite = sendto(sock, message, sizeof(message), (void *)&addr, sizeof(addr)));

	DEFINE_UNIX_SOCKADDR(addr2, "/sock3-1.sock");
	STRICT(bind(sock, (void *)&addr2, sizeof(addr2)));
	STRICT(nwrite = sendto(sock, message, sizeof(message), (void *)&addr, sizeof(addr)));

	return 0;
}

int dgram_send_recv_server(Barrier *barrier) {
	int sock;

	STRICT(sock = socket(PF_LOCAL, SOCK_DGRAM, 0));
	DEFINE_UNIX_SOCKADDR(addr, "/sock3.sock");

	STRICT(bind(sock, (void *)&addr, sizeof(addr)));
	barrier_release(barrier);

	char buf[128];
	struct sockaddr_un client_addr = {
	    .family = PF_LOCAL,
	};

	ssize_t nread;
	socklen_t client_addr_len = sizeof(client_addr);
	STRICT(nread = recvfrom(sock, buf, sizeof(buf), (void *)&client_addr, &client_addr_len));

	ft_printf("%s", buf);
	ft_printf("addr = %s\n", client_addr.data);

	client_addr_len = sizeof(client_addr);
	STRICT(nread = recvfrom(sock, buf, sizeof(buf), (void *)&client_addr, &client_addr_len));

	ft_printf("%s", buf);
	ft_printf("addr = %s\n", client_addr.data);

	return 0;
}

typedef struct _TestCase {
	int (*server)(Barrier *);
	int (*client)(Barrier *);
	const char *test_name;
} TestCase;

int is_null_test_case(TestCase *tc) {
	return (tc->client == NULL || tc->server == NULL || tc->test_name == NULL);
}

static TestCase test_array[] = {
    {
	.server = dgram_basic_io_server,
	.client = dgram_basic_io_client,
	.test_name = "DGRAM basic I/O",
    },
    {
	.server = stream_basic_io_server,
	.client = stream_basic_io_client,
	.test_name = "STREAM basic I/O",
    },
    {
	.server = dgram_send_recv_server,
	.client = dgram_send_recv_client,
	.test_name = "DGRAM sendto recvfrom",
    },
    {
	.server = NULL,
	.client = NULL,
	.test_name = NULL,
    },
};

int check_test_result(char *who, int status) {
	int result = 1;
	ft_printf("> %s: ", who);
	if (WIFEXITED(status)) {
		result = WEXITSTATUS(status);
		ft_printf("exited with = %d\n", result);
	} else if (WIFSIGNALED(status)) {
		result = WTERMSIG(status);
		ft_printf("signaled with = %d\n", result);
	} else {
		ft_printf("terminated.\n");
	}

	return result;
}

int main(void) {
	for (TestCase *tc = test_array; !is_null_test_case(tc); ++tc) {
		ft_printf("\n> RUN: %s\n\n", tc->test_name);

		pid_t server, client;
		Barrier barrier;

		barrier_init(&barrier);
		STRICT(server = fork());
		if (server == 0) {
			_exit(tc->server(&barrier));
		}

		STRICT(client = fork());
		if (client == 0) {
			_exit(tc->client(&barrier));
		}

		barrier_destroy(&barrier);

		int status;

		STRICT(waitpid(server, &status, 0));
		if (check_test_result("server", status)) {
			ft_printf("> TEST failed.\n");
			while (1)
				;
		}

		STRICT(waitpid(client, &status, 0));
		if (check_test_result("client", status)) {
			ft_printf("> TEST failed.\n");
			while (1)
				;
		}

		ft_printf("> TEST passed.\n");
	}
}