#include <fcntl.h>
#include <signal.h>
#include <unistd.h>

#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/wait.h>

#include "kfs/ft.h"
#include "kfs/internal/prelude.h"
#include "kfs/kernel.h"
#include "kfs/libft.h"
#include "sys/mount.h"
#include "sys/socket.h"

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

#define MIN(x, y) ((x) < (y) ? (x) : (y))

// x must be const char * literal!!
#define STREQ(x, y, len) (((len) >= sizeof(x) - 1) && (ft_strncmp((x), (y), sizeof(x) - 1) == 0))

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

void mkfile_name(char *buf, int index) {
	int prefix_len = 2;
	buf[0] = 't';
	buf[1] = 'f';

	int copy = index;
	int digit = 0;
	while (copy > 0) {
		digit++;
		copy /= 10;
	}

	buf[prefix_len + digit] = 0;

	while (digit > 0) {
		buf[prefix_len + digit - 1] = '0' + index % 10;
		index /= 10;
		digit--;
	}
}

void builtin_ntouch(int idx) {
	char buf[4096];

	idx = extract(idx, buf);

	int count = ft_atoi(buf);

	for (int i = 0; i < count; i++) {
		mkfile_name(buf, i);
		int fd = open(buf, O_CREAT | O_EXCL, 0777);
		if (fd < 0) {
			show_error("touch: ntouch", fd);
			return;
		}
		close(fd);
	}
}

void builtin_power_off() {
	reboot(KFS_POWER_OFF);
}

void builtin_reboot() {
	reboot(KFS_RESTART);
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

	int len = 8;
	char *b = "01234567";

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

void builtin_tc(int idx) {
	char buf[4096];
	char buf2[4096];

	idx = extract(idx, buf);
	idx = ignore_ws(idx);
	idx = extract(idx, buf2);

	int len = ft_atoi(buf2);

	int ret = truncate(buf, len);
	if (ret < 0) {
		show_error("tc:", ret);
	} else {
		ft_printf("%s truncated to %d\n", buf, len);
	}
}

void builtin_lc(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int fd = open(buf, O_RDWR);
	if (fd < 0) {
		show_error("lc: open", fd);
		return;
	}

	int ret = read(fd, buf, 4096);
	int total_size = 0;
	while (ret > 0) {
		total_size += ret;
		ret = read(fd, buf, 4096);
		if (ret < 0) {
			show_error("lc: read", ret);
		}
	}

	ft_printf("letter count: %d\n", total_size);
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

void builtin_timespec() {
	struct timespec t;

	int fd = open("/dev/timestamp", O_RDONLY);

	int ret = read(fd, &t, sizeof(struct timespec));

	if (ret < 0) {
		ft_printf("Device not present\n");
	} else {
		ft_printf("second: %d, nano second: %d\n", t.tv_sec, t.tv_nsec);
	}
	close(fd);
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

	ft_printf("  uid: %d\n  gid: %d\n  size: %d\n  mode: ", st.uid, st.gid, st.size);
	ft_putnbr_o(st.perm);
	ft_putstr("\n  type: ");
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
	char dev_path[1024];
	char mount_point[1024];
	char fs_name[1024];

	idx = extract(idx, dev_path);
	idx = ignore_ws(idx);

	idx = extract(idx, mount_point);
	idx = ignore_ws(idx);

	idx = extract(idx, fs_name);
	idx = ignore_ws(idx);

	int ret = mount(dev_path, mount_point, fs_name);
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

void builtin_pwd(void) {
	char buf[4096];

	ft_printf("%s\n", getcwd(buf, sizeof(buf)));
}

void builtin_test(void) {
	execve("/bin/test", NULL, NULL);
}

void builtin_insmod(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int ret = init_module(buf);
	if (ret != 0) {
		show_error("insmod: init_module", ret);
	}
}

void builtin_rmmod(int idx) {
	char buf[4096];

	idx = extract(idx, buf);
	int ret = cleanup_module(buf);
	if (ret != 0) {
		show_error("rmmod: cleanup_module", ret);
	}
}

void builtin_exec(int idx) {
	char buf[4096];

	idx = extract(idx, buf);

	pid_t pid = fork();

	if (pid == 0) {

		int ret = execve(buf, NULL, NULL);

		if (ret < 0) {
			show_error("rmmod: cleanup_module", ret);
		}
	} else {
		int stat = 0;

		signal(SIGINT, SIG_IGN);
		signal(SIGQUIT, SIG_IGN);

		waitpid(pid, &stat, 0);

		signal(SIGINT, SIG_DFL);
		signal(SIGQUIT, SIG_DFL);
	}
}

void builtin_env(char **envp) {
	for (char **p = envp; *p; ++p) {
		ft_printf("%s\n", *p);
	}
}

void builtin_lsmod() {
	char buf[4096];

	int fd = open("/sys/modules", O_DIRECTORY | O_RDONLY | O_CLOEXEC, 0777);
	if (fd < 0) {
		show_error("lsmod: open", fd);
		return;
	}

	int end = getdents(fd, buf, 4096);
	int curr = 0;
	while (curr < end) {
		struct kfs_dirent *dir = (struct kfs_dirent *)&buf[curr];

		if (dir->name[0] != '.')
			ft_printf("%s\n", dir->name);
		curr += dir->size;
	}
	close(fd);
}

int main(int argc, char **argv, char **envp) {
	ft_printf("====== sh ======\n");
	ft_printf(" pid = %d\n", getpid());
	ft_printf(" sid = %d\n", getsid(0));
	ft_printf(" argc = %d\n", argc);
	for (char **p = argv; *p; ++p)
		ft_printf(" argv = %s\n", *p);

	for (;;) {
		ft_putstr("sh==> ");
		unsigned int line_len = getline();

		if (STREQ("env", line_buf, line_len)) {
			builtin_env(envp);
		} else if (STREQ("insmod", line_buf, line_len)) {
			builtin_insmod(ignore_ws(6));
		} else if (STREQ("rmmod", line_buf, line_len)) {
			builtin_rmmod(ignore_ws(5));
		} else if (STREQ("lsmod", line_buf, line_len)) {
			builtin_lsmod();
		} else if (STREQ("ls", line_buf, line_len)) {
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
		} else if (STREQ("lc", line_buf, line_len)) {
			builtin_lc(ignore_ws(2));
		} else if (STREQ("tc", line_buf, line_len)) {
			builtin_tc(ignore_ws(2));
		} else if (STREQ("timespec", line_buf, line_len)) {
			builtin_timespec();
		} else if (STREQ("poweroff", line_buf, line_len)) {
			builtin_power_off();
		} else if (STREQ("reboot", line_buf, line_len)) {
			builtin_reboot();
		} else if (STREQ("ntouch", line_buf, line_len)) {
			builtin_ntouch(ignore_ws(6));
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
		} else if (STREQ("pwd", line_buf, line_len)) {
			builtin_pwd();
		} else if (STREQ("test", line_buf, line_len)) {
			builtin_test();
		} else if (STREQ("exec", line_buf, line_len)) {
			builtin_exec(ignore_ws(4));
		} else if (STREQ("exit", line_buf, line_len)) {
			break;
		} else {
			extract(0, line_buf);
			ft_putstr("sh: ");
			ft_putstr(line_buf);
			ft_putstr(": unknown command.\n");
		}
	}

	return 0;
}
