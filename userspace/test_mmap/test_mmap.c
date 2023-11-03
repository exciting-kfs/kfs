#include "kfs/libft.h"
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/wait.h>

int main(void) {
	int fd = open("/root/hello_mmap", O_RDWR | O_CREAT, 0666);
	ft_printf("fd: %d\n", fd);

	write(fd, "hello_mmap!\n\0", 13);

	void *mmaped = mmap((void *)0x10000, 4096, PROT_WRITE | PROT_READ, MMAP_SHARED, fd, 0);

	ft_printf("pid: %d :%s", getpid(), mmaped);

	pid_t pid = fork();

	if (pid == 0) {
		ft_printf("pid: %d :%s", getpid(), mmaped);
		return 0;
	} else {
		int stat;
		waitpid(pid, &stat, 0);

		munmap(mmaped, 4096);

		ft_printf("pid: %d :%s", getpid(), mmaped);
	}
}