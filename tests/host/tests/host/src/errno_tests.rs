//! Errno value tests

const EPERM: i64 = 1;
const ENOENT: i64 = 2;
const ESRCH: i64 = 3;
const EINTR: i64 = 4;
const EIO: i64 = 5;
const EBADF: i64 = 9;
const ECHILD: i64 = 10;
const EAGAIN: i64 = 11;
const ENOMEM: i64 = 12;
const EACCES: i64 = 13;
const EFAULT: i64 = 14;
const EEXIST: i64 = 17;
const EINVAL: i64 = 22;
const ENOSYS: i64 = 38;
const ERANGE: i64 = 34;

#[test]
fn errno_values_are_positive() {
    assert!(EPERM > 0);
    assert!(ENOENT > 0);
    assert!(ENOSYS > 0);
}

#[test]
fn errno_negation_for_syscall_return() {
    assert_eq!(-ENOENT, -2);
    assert_eq!(-EFAULT, -14);
    assert_eq!(-ENOSYS, -38);
}

#[test]
fn errno_values_unique() {
    let errs = [EPERM, ENOENT, ESRCH, EINTR, EIO, EBADF, ECHILD, EAGAIN,
                ENOMEM, EACCES, EFAULT, EEXIST, EINVAL, ENOSYS, ERANGE];
    for i in 0..errs.len() {
        for j in (i+1)..errs.len() {
            assert_ne!(errs[i], errs[j], "Duplicate errno values at {}, {}", i, j);
        }
    }
}

#[test]
fn errno_linux_compat_eperm() { assert_eq!(EPERM, 1); }
#[test]
fn errno_linux_compat_enoent() { assert_eq!(ENOENT, 2); }
#[test]
fn errno_linux_compat_esrch() { assert_eq!(ESRCH, 3); }
#[test]
fn errno_linux_compat_eintr() { assert_eq!(EINTR, 4); }
#[test]
fn errno_linux_compat_eio() { assert_eq!(EIO, 5); }
#[test]
fn errno_linux_compat_ebadf() { assert_eq!(EBADF, 9); }
#[test]
fn errno_linux_compat_echild() { assert_eq!(ECHILD, 10); }
#[test]
fn errno_linux_compat_eagain() { assert_eq!(EAGAIN, 11); }
#[test]
fn errno_linux_compat_enomem() { assert_eq!(ENOMEM, 12); }
#[test]
fn errno_linux_compat_eacces() { assert_eq!(EACCES, 13); }
#[test]
fn errno_linux_compat_efault() { assert_eq!(EFAULT, 14); }
#[test]
fn errno_linux_compat_eexist() { assert_eq!(EEXIST, 17); }
#[test]
fn errno_linux_compat_einval() { assert_eq!(EINVAL, 22); }
#[test]
fn errno_linux_compat_erange() { assert_eq!(ERANGE, 34); }
#[test]
fn errno_linux_compat_enosys() { assert_eq!(ENOSYS, 38); }
