#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "kfs/ft.h"
#include "kfs/kernel.h"

int main(void) {

	// mkdir("/dir1", 0777);
	// mkdir("/dir2", 0777);
	int fd = open("/abc", O_CREAT | O_EXCL | O_RDWR, 0777);
	const char buf[] = "0123456789";
	write(fd, buf, 10);
	close(fd);
	truncate("/abc", 5);
	fd = open("/abc", O_RDONLY);

	char ch;
	int ret;
	while ((ret = read(fd, &ch, 1)) > 0) {
		write(1, &ch, 1);
	}
	write(1, "\n", 1);
	// chdir("/dir1");
	// int root_fd = open2("..", O_RDWR | O_DIRECTORY | O_CLOEXEC, 0777);

	// char buf[4096];
	// int end = getdents(root_fd, buf, 4096);
	// int curr = 0;

	// while (curr < end) {
		// struct kfs_dirent *dir = (struct kfs_dirent *)&buf[curr];
		// ft_putstr(dir->name);
		// ft_putstr("\n");
		// curr += dir->size;
	// }

	return 0;
}