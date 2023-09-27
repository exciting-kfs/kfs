#ifndef _SYS_SOCKET_H
#define _SYS_SOCKET_H

#include "kfs/internal/prelude.h"

struct sockaddr {
	unsigned short family;
	char data[0];
};

typedef unsigned int socklen_t;

#define PF_LOCAL 0

#define SOCK_STREAM 1
#define SOCK_DGRAM 2

int socket(int domain, int type, int protocol);
int bind(int socket, const struct sockaddr *address, int address_len);
int connect(int socket, const struct sockaddr *address, int address_len);
int accept(int socket, struct sockaddr *address, int *address_len);
int listen(int socket, int backlog);
ssize_t sendto(int socket, const void *buffer, size_t length, const struct sockaddr *dest_addr,
	       socklen_t dest_len);
ssize_t recvfrom(int socket, void *buffer, size_t length, struct sockaddr *address,
		 socklen_t *address_len);

#endif // _SYS_SOCKET_H
