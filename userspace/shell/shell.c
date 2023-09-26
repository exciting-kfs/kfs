#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include "kfs/ft.h"
#include "kfs/internal/prelude.h"
#include "kfs/kernel.h"
#include "kfs/libft.h"
#include "sys/mount.h"

char line_buf[8192] = {42};

void show_error(const char *where, int err) {
	ft_printf("shell: %s: %d\n", where, err);
}

void panic(const char *where, int err) {
	show_error(where, err);
	_exit(1);
}

int getline(void) {
	int ret;

	int cursor = 0;
	while ((ret = read(0, line_buf + cursor, 1)) > 0) {
		if (line_buf[cursor] == '\n') {
			line_buf[cursor] = '\0';
			return cursor;
		}
		cursor += 1;
	}

	// ret == 0 case ?
	panic("getline - read", ret);
	return 0;
}

int streq(const char *a, const char *b, int len) {
	for (int i = 0; i < len; i++) {
		if (a[i] != b[i]) {
			return 0;
		}
	}
	return 1;
}

#define MIN(x, y) ((x) < (y) ? (x) : (y))

// x must be const char * literal!!
#define STREQ(x, y, len) (streq((x), (y), MIN(sizeof(x) - 1, (len))))

int check(char ch, const char *set) {
	while (*set) {
		if (ch == *set) {
			return 1;
		}
		set += 1;
	}
	return 0;
}

int ignore_ws(int idx) {
	while (check(line_buf[idx], "\n ")) {
		idx += 1;
	}

	return idx;
}

int extract(int idx, char *buf) {
	int i = 0;
	while (line_buf[idx + i] && !check(line_buf[idx + i], "\n ")) {
		buf[i] = line_buf[idx + i];
		i += 1;
	}
	buf[i] = '\0';

	return idx + i;
}

void builtin_ls(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	if (buf[0] == '\0') {
		buf[0] = '.';
		buf[1] = '\0';
	}
	int fd = open(buf, O_DIRECTORY | O_RDONLY | O_CLOEXEC, 0777);
	if (fd < 0) {
		show_error("ls: open", fd);
		return;
	}

	int end = getdents(fd, buf, 4096);
	int curr = 0;
	while (curr < end) {
		struct kfs_dirent *dir = (struct kfs_dirent *)&buf[curr];

		ft_putstr(dir->name);
		ft_putstr("\n");

		curr += dir->size;
	}
	close(fd);
}

void builtin_cd(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int ret = chdir(buf);
	if (ret < 0) {
		show_error("cd: chdir", ret);
	}
}

void builtin_cat(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int fd = open(buf, O_RDONLY);
	if (fd < 0) {
		show_error("cat: open", fd);
		return;
	}

	char ch;
	int ret;
	while ((ret = read(fd, &ch, 1)) > 0) {
		write(1, &ch, 1);
	}
	write(1, "\n", 1);

	if (ret != 0) {
		show_error("cat: read", ret);
	}
	close(fd);
}

void builtin_touch(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int fd = open(buf, O_CREAT | O_EXCL, 0777);
	if (fd < 0) {
		show_error("touch: open", fd);
		return;
	}
	close(fd);
}

void builtin_mkdir(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int fd = mkdir(buf, 0777);
	if (fd < 0) {
		show_error("mkdir: mkdir", fd);
	}
}

void builtin_write(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int fd = open(buf, O_WRONLY);
	if (fd < 0) {
		show_error("write: open", fd);
		return;
	}

	idx = ignore_ws(idx);
	idx = extract(idx, buf);
	for (char *p = buf; *p; p++) {
		int ret = write(fd, p, 1);
		if (ret < 0) {
			show_error("write: write", ret);
			break;
		}
	}
	close(fd);
}

void builtin_wf(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int fd = open(buf, O_WRONLY);
	if (fd < 0) {
		show_error("wf: open", fd);
		return;
	}

	idx = ignore_ws(idx);
	idx = extract(idx, buf);

	int size = ft_atoi(buf);

	int len = 10;
	char *b = "0123456789";

	while (size > 0) {
		int l = size < len ? size : len;
		int ret = write(fd, b, l);
		if (ret < 0) {
			show_error("wf: write", ret);
			break;
		}
		size -= len;
	}

	close(fd);
}

void builtin_rmdir(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int ret = rmdir(buf);

	if (ret != 0) {
		show_error("rmdir: rmdir", ret);
		return;
	}
}

void builtin_rm(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int ret = unlink(buf);

	if (ret != 0) {
		show_error("rm: unlink", ret);
		return;
	}
}

void builtin_stat(int idx) {
	char buf[4096];

	idx = extract(idx, buf);

	struct stat st;
	int ret = stat(buf, &st);
	if (ret != 0) {
		show_error("stat: stat", ret);
		return;
	}

	ft_printf("\tuid: %d\n\tgid: %d\n\tsize: %d\n\tmode: 0x%x\n\ttype: ", st.uid, st.gid,
		  st.size, st.perm);
	switch (st.file_type) {
	case 1:
		ft_printf("regular file\n");
		break;
	case 2:
		ft_printf("directory\n");
		break;
	case 3:
		ft_printf("character special\n");
		break;
	case 4:
		ft_printf("block special\n");
		break;
	case 5:
		ft_printf("fifo\n");
		break;
	case 6:
		ft_printf("socket\n");
		break;
	case 7:
		ft_printf("symbolic link\n");
		break;
	default:
		ft_printf("unknown\n");
		break;
	}
}

void builtin_chmod(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	if (ft_strlen(buf) != 3) {
		ft_putstr("chmod: invalid mode\n");
		return;
	}
	mode_t mode = (buf[0] - '0') * 64 + (buf[1] - '0') * 8 + (buf[2] - '0');

	idx = ignore_ws(idx);
	idx = extract(idx, buf);
	int ret = chmod(buf, mode);
	if (ret != 0) {
		show_error("chmod: chmod", ret);
		return;
	}
}

int atoi_naive(const char *s) {
	if (*s == '\0') {
		return 0;
	}

	return (atoi_naive(s + 1) * 10) + (*s - '0');
}

void builtin_chown(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	uid_t owner = atoi_naive(buf);

	idx = ignore_ws(idx);
	idx = extract(idx, buf);
	gid_t group = atoi_naive(buf);

	idx = ignore_ws(idx);
	idx = extract(idx, buf);
	int ret = chown(buf, owner, group);

	if (ret != 0) {
		show_error("chown: chown", ret);
	}
}

void builtin_mount(int idx) {
	char buf1[2048];
	char buf2[2048];

	idx = extract(idx, buf1);
	idx = ignore_ws(idx);

	idx = extract(idx, buf2);
	idx = ignore_ws(idx);

	int ret = mount(buf1, buf2);
	if (ret < 0) {
		show_error("mount: mount", ret);
	}
}

void builtin_umount(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int ret = umount(buf);
	if (ret != 0) {
		show_error("umount: umount", ret);
	}
}

void builtin_symlink(int idx) {
	char buf1[2048];
	char buf2[2048];

	idx = extract(idx, buf1);
	idx = ignore_ws(idx);

	idx = extract(idx, buf2);
	idx = ignore_ws(idx);

	int ret = symlink(buf1, buf2);
	if (ret < 0) {
		show_error("symlink: symlink", ret);
	}
}

int main(void) {
	ft_printf("%c,%c,%c, hello!\n", 'a', 'b', 'c');

	for (;;) {
		ft_putstr("sh==> ");
		unsigned int line_len = getline();

		if (STREQ("ls", line_buf, line_len)) {
			builtin_ls(ignore_ws(2));
		} else if (STREQ("cd", line_buf, line_len)) {
			builtin_cd(ignore_ws(2));
		} else if (STREQ("cat", line_buf, line_len)) {
			builtin_cat(ignore_ws(3));
		} else if (STREQ("touch", line_buf, line_len)) {
			builtin_touch(ignore_ws(5));
		} else if (STREQ("mkdir", line_buf, line_len)) {
			builtin_mkdir(ignore_ws(5));
		} else if (STREQ("write", line_buf, line_len)) {
			builtin_write(ignore_ws(5));
		} else if (STREQ("wf", line_buf, line_len)) {
			builtin_wf(ignore_ws(2));
		} else if (STREQ("rmdir", line_buf, line_len)) {
			builtin_rmdir(ignore_ws(5));
		} else if (STREQ("rm", line_buf, line_len)) {
			builtin_rm(ignore_ws(2));
		} else if (STREQ("stat", line_buf, line_len)) {
			builtin_stat(ignore_ws(4));
		} else if (STREQ("chmod", line_buf, line_len)) {
			builtin_chmod(ignore_ws(5));
		} else if (STREQ("chown", line_buf, line_len)) {
			builtin_chown(ignore_ws(5));
		} else if (STREQ("mount", line_buf, line_len)) {
			builtin_mount(ignore_ws(5));
		} else if (STREQ("umount", line_buf, line_len)) {
			builtin_umount(ignore_ws(6));
		} else if (STREQ("symlink", line_buf, line_len)) {
			builtin_symlink(ignore_ws(7));
		} else {
			extract(0, line_buf);
			ft_putstr("sh: ");
			ft_putstr(line_buf);
			ft_putstr(": unknown command.\n");
		}
	}

	return 0;
}
