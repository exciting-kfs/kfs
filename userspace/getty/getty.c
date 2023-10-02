#include <fcntl.h>
#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/wait.h>

#include "kfs/ft.h"
#include "kfs/libft.h"

// clang-format off
const char passwd_file[] =
    "root::0:0::/root:shell.bin\n"
    "cjeon:$6$xKfGiVIDU2eFHpz9$CIrn5g9ODPQM1VznJ941RjEeoPvaKNHak1o7rrUJR1jXg/kZL7bmQcv5xD3GFLCn39dhWRlsMmbNam59tDIgh0:1000:1000::/home/cjeon:shell.bin\n";
// clang-format on

char *__crypt_sha512(const char *key, const char *setting, char *output);

#define STRICT(expr)                                                                               \
	do {                                                                                       \
		int __ret = (expr);                                                                \
		if (__ret < 0) {                                                                   \
			ft_printf("%s:%d: [%s]: return was: %d\n", __FILE__, __LINE__, #expr,      \
				  __ret);                                                          \
			_exit(1);                                                                  \
		}                                                                                  \
	} while (0)

typedef struct {
	unsigned int alloc_size;
	unsigned char data[0];
} AllocInfo;

static void *malloc_naive(size_t size) {
	size += sizeof(AllocInfo);
	if (size % 4096 != 0)
		size = (size + 4095) & ~(4095);

	AllocInfo *mem = mmap(NULL, size, PROT_READ | PROT_WRITE, MMAP_PRIVATE, -1, 0);
	if (!mem)
		return NULL;

	mem->alloc_size = size;

	return &mem->data;
}

static void free_naive(void *p) {
	AllocInfo *ai = (void *)(((unsigned char *)p) - 4);

	munmap(ai, ai->alloc_size);
}

#define PP_EOF (-1)
#define PP_ERR (-2)

typedef enum {
	PP_START = 1,
	PP_NAME,
	PP_PW,
	PP_UID,
	PP_GID,
	PP_COMMENT,
	PP_HOME_DIR,
	PP_SHELL,
} PPStatus;

typedef struct {
	int fd;
	char next_char;
	PPStatus state;
} PasswdParser;

typedef struct {
	char *name;
	char *pw;
	int uid;
	int gid;
	char *comment;
	char *home;
	char *shell;
} PasswdEnt;

void pp_pop(PasswdParser *self) {
	if (self->next_char == PP_EOF || self->next_char == PP_ERR)
		return;

	ssize_t nread = read(self->fd, &self->next_char, 1);
	if (nread == 0)
		self->next_char = PP_EOF;
	else if (nread < 0 || self->next_char < 0)
		self->next_char = PP_ERR;
}

int pp_init(PasswdParser *self) {
	self->fd = open("/etc/passwd", O_RDONLY);
	if (self->fd < 0)
		return -1;

	ssize_t nread = read(self->fd, &self->next_char, 1);

	if (nread == 0)
		self->next_char = PP_EOF;
	else if (nread < 0 || self->next_char < 0)
		self->next_char = PP_ERR;

	self->state = PP_START;

	return 0;
}

char pp_peek(PasswdParser *self) {
	return self->next_char;
}

void pp_drop(PasswdParser *self) {
	close(self->fd);
}

void pent_drop(PasswdEnt *self) {
	if (self == NULL) {
		return;
	}

	free_naive(self->name);
	free_naive(self->pw);
	free_naive(self);
}

#define PENT_BUFFER_SIZE 2048

PasswdEnt *pent_new(void) {
	PasswdEnt *ent = malloc_naive(sizeof(*ent));
	ent->name = malloc_naive(PENT_BUFFER_SIZE);
	ent->pw = malloc_naive(PENT_BUFFER_SIZE);
	ent->uid = -1;
	ent->gid = -1;
	ent->comment = malloc_naive(PENT_BUFFER_SIZE);
	ent->home = malloc_naive(PENT_BUFFER_SIZE);
	ent->shell = malloc_naive(PENT_BUFFER_SIZE);

	return ent;
}

int is_valid_field(char ch) {
	return ft_isprint(ch);
}

#define ID_MAX 1000000

int pp_get_entry(PasswdParser *self, PasswdEnt **entry) {

	PasswdEnt *ent = pent_new();

	size_t len = 0;
	int ids = 0;
	int err = 0;
	char ch;
	for (;;) {
		switch (self->state) {
		case PP_START:
			ch = pp_peek(self);
			if (ch == '\n') {
				pp_pop(self);
			} else if (ch == PP_EOF) {
				err = 0;
				goto parse_error;
			} else if (ch == PP_ERR) {
				err = -PP_START;
				goto parse_error;
			} else {
				self->state = PP_NAME;
				len = 0;
			}
			break;
		case PP_NAME:
			ch = pp_peek(self);
			if (ch == ':') {
				self->state = PP_PW;
				ent->name[len] = '\0';
				len = 0;
			} else if (is_valid_field(ch) && len < (PENT_BUFFER_SIZE - 1)) {
				ent->name[len] = ch;
				len += 1;
			} else {
				err = -PP_NAME;
				goto parse_error;
			}
			pp_pop(self);
			break;
		case PP_PW:
			ch = pp_peek(self);
			if (ch == ':') {
				self->state = PP_UID;
				ent->pw[len] = '\0';
				ent->uid = 0;
				len = 0;
			} else if (is_valid_field(ch) && len < (PENT_BUFFER_SIZE - 1)) {
				ent->pw[len] = ch;
				len += 1;
			} else {
				err = -PP_PW;
				goto parse_error;
			}
			pp_pop(self);
			break;
		case PP_UID:
			ch = pp_peek(self);
			if (ch == ':' && len != 0) {
				self->state = PP_GID;
				ent->gid = 0;
				len = 0;
			} else if (ft_isdigit(ch) && ids * 10 + (ch - '0') < ID_MAX) {
				ent->uid = ent->uid * 10 + (ch - '0');
				len += 1;
			} else {
				err = -PP_UID;
				goto parse_error;
			}
			pp_pop(self);
			break;
		case PP_GID:
			ch = pp_peek(self);
			if (ch == ':' && len != 0) {
				self->state = PP_COMMENT;
				len = 0;
			} else if (ft_isdigit(ch) && ids * 10 + (ch - '0') < ID_MAX) {
				ent->gid = ent->gid * 10 + (ch - '0');
				len += 1;
			} else {
				err = -PP_GID;
				goto parse_error;
			}
			pp_pop(self);
			break;
		case PP_COMMENT:
			ch = pp_peek(self);
			if (ch == ':') {
				self->state = PP_HOME_DIR;
				ent->comment[len] = '\0';
				len = 0;
			} else if (is_valid_field(ch) && len < (PENT_BUFFER_SIZE - 1)) {
				ent->comment[len] = ch;
				len += 1;
			} else {
				err = -PP_COMMENT;
				goto parse_error;
			}
			pp_pop(self);
			break;
		case PP_HOME_DIR:
			ch = pp_peek(self);
			if (ch == ':') {
				self->state = PP_SHELL;
				ent->home[len] = '\0';
				len = 0;
			} else if (is_valid_field(ch) && len < (PENT_BUFFER_SIZE - 1)) {
				ent->home[len] = ch;
				len += 1;
			} else {
				err = -PP_HOME_DIR;
				goto parse_error;
			}
			pp_pop(self);
			break;
		case PP_SHELL:
			ch = pp_peek(self);
			if (ch == PP_EOF || ch == '\n') {
				self->state = PP_START;
				ent->shell[len] = '\0';
				*entry = ent;
				return 0;
			} else if (is_valid_field(ch) && len < (PENT_BUFFER_SIZE - 1)) {
				ent->shell[len] = ch;
				len += 1;
			} else {
				err = -PP_SHELL;
				goto parse_error;
			}
			pp_pop(self);
		}
	}

parse_error:
	pent_drop(ent);
	*entry = NULL;
	return err;
}

int auth_sha512(const char *pw, const char *pw_ent) {
	if (pw_ent[0] == '$' && pw_ent[1] == '6' && pw_ent[2] == '$') {
		size_t len = ft_strrchr(pw_ent, '$') - pw_ent;
		char setting[2048];
		ft_memcpy(setting, pw_ent, len);
		setting[len] = '\0';

		char new_secret[128];
		__crypt_sha512(pw, setting, new_secret);

		return ft_strncmp(pw_ent, new_secret, 128) == 0;
	}

	return 0;
}

size_t getline_prompt(char *buf, size_t len, const char *prompt) {
	size_t nread;
	ft_putstr(prompt);
	STRICT(nread = read(0, buf, len - 1));
	buf[--nread] = '\0';

	return nread;
}

PasswdEnt *try_login() {
	char username[PENT_BUFFER_SIZE];
	getline_prompt(username, sizeof(username), "username: ");

	char password[PENT_BUFFER_SIZE];
	getline_prompt(password, sizeof(password), "password: ");

	PasswdParser pp;
	PasswdEnt *ent;

	STRICT(pp_init(&pp));

	for (;;) {
		if (pp_get_entry(&pp, &ent))
			break;
		if (ent == NULL)
			break;

		if (ft_strncmp(username, ent->name, ft_strlen(ent->name)) == 0) {
			if (ent->pw[0] == '\0' || auth_sha512(password, ent->pw))
				return ent;
			else
				break;
		}

		pent_drop(ent);
	}

	pent_drop(ent);
	pp_drop(&pp);

	return NULL;
}

void get_login_shell() {
	PasswdEnt *ent;
	while (!(ent = try_login())) {
		ft_printf("invalid PW or USERNAME. try again\n");
	}

	ft_printf("LOGIN\n  user: %s\n  uid: %d\n  gid: %d\n  comment: %s\n  home: "
		  "%s\n  shell: %s\n",
		  ent->name, ent->uid, ent->gid, ent->comment, ent->home, ent->shell);

	int pid = fork();
	if (pid == 0) {
		signal(SIGINT, SIG_DFL);
		signal(SIGQUIT, SIG_DFL);
		setuid(ent->uid);
		setgid(ent->gid);
		exec(ent->shell);
		_exit(128);
	}

	int status;
	waitpid(pid, &status, 0);
}

int main(void) {
	int passwd;

	mkdir("/etc", 0755);
	passwd = open("/etc/passwd", O_CREAT | O_EXCL | O_WRONLY, 0644);
	write(passwd, passwd_file, sizeof(passwd_file) - 1);

	close(passwd);

	for (;;)
		get_login_shell();
}
