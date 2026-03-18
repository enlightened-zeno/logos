/// POSIX-compatible error numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
#[allow(clippy::upper_case_acronyms)]
pub enum Errno {
    Success = 0,
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EPIPE = 32,
    ERANGE = 34,
    ENOSYS = 38,
    ENOTEMPTY = 39,
    ENAMETOOLONG = 36,
}

impl Errno {
    /// Convert to the negative error value returned to userspace.
    pub fn as_neg(self) -> i64 {
        -(self as i64)
    }
}

/// Syscall result type: positive values are success, negative are -errno.
pub type SyscallResult = i64;
