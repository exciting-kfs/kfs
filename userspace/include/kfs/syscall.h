#ifndef _KFS_SYSCALL_H
#define _KFS_SYSCALL_H

// see also: https://gcc.gnu.org/onlinedocs/gcc/Machine-Constraints.html

static inline long __syscall0(long n) {
	unsigned long __ret;
	__asm__ __volatile__("int $0x80" : "=a"(__ret) : "a"(n) : "memory");
	return __ret;
}

static inline long __syscall1(long n, long a1) {
	unsigned long __ret;
	__asm__ __volatile__("int $0x80" : "=a"(__ret) : "a"(n), "b"(a1) : "memory");
	return __ret;
}

static inline long __syscall2(long n, long a1, long a2) {
	unsigned long __ret;
	__asm__ __volatile__("int $0x80" : "=a"(__ret) : "a"(n), "b"(a1), "c"(a2) : "memory");
	return __ret;
}

static inline long __syscall3(long n, long a1, long a2, long a3) {
	unsigned long __ret;
	__asm__ __volatile__("int $0x80"
			     : "=a"(__ret)
			     : "a"(n), "b"(a1), "c"(a2), "d"(a3)
			     : "memory");
	return __ret;
}

static inline long __syscall4(long n, long a1, long a2, long a3, long a4) {
	unsigned long __ret;
	__asm__ __volatile__("int $0x80"
			     : "=a"(__ret)
			     : "a"(n), "b"(a1), "c"(a2), "d"(a3), "S"(a4)
			     : "memory");
	return __ret;
}

static inline long __syscall5(long n, long a1, long a2, long a3, long a4, long a5) {
	unsigned long __ret;
	__asm__ __volatile__("int $0x80"
			     : "=a"(__ret)
			     : "a"(n), "b"(a1), "c"(a2), "d"(a3), "S"(a4), "D"(a5)
			     : "memory");
	return __ret;
}

static inline long __syscall6(long n, long a1, long a2, long a3, long a4, long a5, long a6) {
	unsigned long __ret;
	__asm__ __volatile__("push %7\n"
			     "push %%ebp\n"
			     "mov 4(%%esp), %%ebp\n"
			     "int $0x80\n"
			     "pop %%ebp\n"
			     "add $4, %%esp"
			     : "=a"(__ret)
			     : "a"(n), "b"(a1), "c"(a2), "d"(a3), "S"(a4), "D"(a5), "g"(a6)
			     : "memory");
	return __ret;
}

#define __do_varg_count(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, xa, xb, xc, xd, xe, xf, n, ...) n
#define __varg_count(...)                                                                          \
	__do_varg_count(, ##__VA_ARGS__, f, e, d, c, b, a, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)

#define __syscall_x(x, n, ...) __syscall##x(n, __VA_ARGS__)
#define ___syscall(x, n, ...) __syscall_x(x, n, __VA_ARGS__)
#define __syscall(n, ...) ___syscall(__varg_count(__VA_ARGS__), n, __VA_ARGS__)

#define __PAIR_MAP_0(f, ...)
#define __PAIR_MAP_2(f, a, b, ...) f(a, b)
#define __PAIR_MAP_4(f, a, b, ...) f(a, b), __PAIR_MAP_2(f, __VA_ARGS__)
#define __PAIR_MAP_6(f, a, b, ...) f(a, b), __PAIR_MAP_4(f, __VA_ARGS__)
#define __PAIR_MAP_8(f, a, b, ...) f(a, b), __PAIR_MAP_6(f, __VA_ARGS__)
#define __PAIR_MAP_a(f, a, b, ...) f(a, b), __PAIR_MAP_8(f, __VA_ARGS__)
#define __PAIR_MAP_c(f, a, b, ...) f(a, b), __PAIR_MAP_a(f, __VA_ARGS__)
#define __PAIR_MAP_x(x, f, ...) __PAIR_MAP_##x(f, __VA_ARGS__)
#define ___PAIR_MAP(x, f, ...) __PAIR_MAP_x(x, f, __VA_ARGS__)
#define __PAIR_MAP(f, ...) ___PAIR_MAP(__varg_count(__VA_ARGS__), f, __VA_ARGS__)

#define __PAIR_GET_A(a, b) a
#define __PAIR_CAST_B(a, b) (long)(b)
#define __PAIR_FLATTEN(a, b) a b

#define __DEFINE_SYSCALL_generic(n, v, r, ...)                                                     \
	static inline r n(__PAIR_MAP(__PAIR_FLATTEN, __VA_ARGS__)) {                               \
		return (r)__syscall(v, __PAIR_MAP(__PAIR_CAST_B, __VA_ARGS__));                    \
	}

#define __DEFINE_SYSCALL_1(n, v, r, x)                                                             \
	static inline r n(x) {                                                                     \
		return (r)__syscall0(v);                                                           \
	}

#define __DEFINE_SYSCALL_2(n, v, r, ...) __DEFINE_SYSCALL_generic(n, v, r, __VA_ARGS__)
#define __DEFINE_SYSCALL_4(n, v, r, ...) __DEFINE_SYSCALL_generic(n, v, r, __VA_ARGS__)
#define __DEFINE_SYSCALL_6(n, v, r, ...) __DEFINE_SYSCALL_generic(n, v, r, __VA_ARGS__)
#define __DEFINE_SYSCALL_8(n, v, r, ...) __DEFINE_SYSCALL_generic(n, v, r, __VA_ARGS__)
#define __DEFINE_SYSCALL_a(n, v, r, ...) __DEFINE_SYSCALL_generic(n, v, r, __VA_ARGS__)
#define __DEFINE_SYSCALL_c(n, v, r, ...) __DEFINE_SYSCALL_generic(n, v, r, __VA_ARGS__)

#define __DEFINE_SYSCALL_x(x, n, v, r, ...) __DEFINE_SYSCALL_##x(n, v, r, __VA_ARGS__)
#define ___DEFINE_SYSCALL(x, n, v, r, ...) __DEFINE_SYSCALL_x(x, n, v, r, __VA_ARGS__)

// DEFINE_SYSCALL: define new syscall.
// ex) DEFINE_SYSCALL(sys_write, 4, ssize_t, int, fd, const void *, buf, size_t, len)
//      will be expanded to
//	    static inline ssize_t sys_write(int fd, const void *buf, size_t len) {
//              return (ssize_t)__syscall3(4, (long)fd, (long)buf, (long)len);
//      }
#define DEFINE_SYSCALL(syscall_name, vec_number, ret_type, ...)                                    \
	___DEFINE_SYSCALL(__varg_count(__VA_ARGS__), syscall_name, vec_number, ret_type,           \
			  __VA_ARGS__)

#endif // _KFS_SYSCALL_H
