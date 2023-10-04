#ifndef _SYS_SOCKET_H
#define _SYS_SOCKET_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

struct sockaddr {
	unsigned short family;
	char data[0];
};

typedef unsigned int socklen_t;

#define PF_LOCAL 0

#define SOCK_STREAM 1
#define SOCK_DGRAM 2

DEFINE_SYSCALL(socket, 359, int, int, domain, int, type, int, protocol);
DEFINE_SYSCALL(bind, 361, int, int, socket, const struct sockaddr *, address, int, address_len);
DEFINE_SYSCALL(connect, 362, int, int, socket, const struct sockaddr *, address, int, address_len);
DEFINE_SYSCALL(accept, 364, int, int, socket, struct sockaddr *, address, int *, address_len);
DEFINE_SYSCALL(listen, 363, int, int, socket, int, backlog);
DEFINE_SYSCALL(sendto, 369, ssize_t, int, socket, const void *, buffer, size_t, length,
	       const struct sockaddr *, dest_addr, socklen_t, dest_len);
DEFINE_SYSCALL(recvfrom, 371, ssize_t, int, socket, void *, buffer, size_t, length,
	       struct sockaddr *, address, socklen_t *, address_len);

#endif // _SYS_SOCKET_H
