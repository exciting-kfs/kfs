use core::alloc::AllocError;

#[repr(isize)]
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Errno {
	UnknownErrno,
	EPERM,
	ENOENT,
	ESRCH,
	EINTR,
	EIO,
	ENXIO,
	E2BIG,
	ENOEXEC,
	EBADF,
	ECHILD,
	EAGAIN,
	ENOMEM,
	EACCES,
	EFAULT,
	ENOTBLK,
	EBUSY,
	EEXIST,
	EXDEV,
	ENODEV,
	ENOTDIR,
	EISDIR,
	EINVAL,
	ENFILE,
	EMFILE,
	ENOTTY,
	ETXTBSY,
	EFBIG,
	ENOSPC,
	ESPIPE,
	EROFS,
	EMLINK,
	EPIPE,
	EDOM,
	ERANGE,
	ENAMETOOLONG,
	ENOTEMPTY,
	ELOOP,
}

impl Errno {
	pub fn as_ret(&self) -> isize {
		-(*self as isize)
	}
}

impl From<AllocError> for Errno {
	fn from(_: AllocError) -> Self {
		Errno::ENOMEM
	}
}

fn desc(errno: Errno) -> &'static str {
	use self::Errno::*;
	match errno {
		UnknownErrno => "Unknown errno",
		EPERM => "Operation not permitted",
		ENOENT => "No such file or directory",
		ESRCH => "No such process",
		EINTR => "Interrupted system call",
		EIO => "I/O error",
		ENXIO => "No such device or address",
		E2BIG => "Argument list too long",
		ENOEXEC => "Exec format error",
		EBADF => "Bad file number",
		ECHILD => "No child processes",
		EAGAIN => "Try again",
		ENOMEM => "Out of memory",
		EACCES => "Permission denied",
		EFAULT => "Bad address",
		ENOTBLK => "Block device required",
		EBUSY => "Device or resource busy",
		EEXIST => "File exists",
		EXDEV => "Cross-device link",
		ENODEV => "No such device",
		ENOTDIR => "Not a directory",
		EISDIR => "Is a directory",
		EINVAL => "Invalid argument",
		ENFILE => "File table overflow",
		EMFILE => "Too many open files",
		ENOTTY => "Not a typewriter",
		ETXTBSY => "Text file busy",
		EFBIG => "File too large",
		ENOSPC => "No space left on device",
		ESPIPE => "Illegal seek",
		EROFS => "Read-only file system",
		EMLINK => "Too many links",
		EPIPE => "Broken pipe",
		EDOM => "Math argument out of domain of func",
		ERANGE => "Math result not representable",
		ENAMETOOLONG => "File name too long",
		ENOTEMPTY => "Directory is not empty",
		ELOOP => "too many levels of symbolic links",
	}
}
