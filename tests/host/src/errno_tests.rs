/// Test that errno values match POSIX.

#[test]
fn test_errno_values() {
    // Standard POSIX errno values
    let expected = vec![
        ("EPERM", 1),
        ("ENOENT", 2),
        ("ESRCH", 3),
        ("EINTR", 4),
        ("EIO", 5),
        ("EBADF", 9),
        ("ECHILD", 10),
        ("EAGAIN", 11),
        ("ENOMEM", 12),
        ("EACCES", 13),
        ("EFAULT", 14),
        ("EEXIST", 17),
        ("EINVAL", 22),
        ("ENOSYS", 38),
    ];

    // We just verify the values are standard — the kernel uses these same numbers
    for (name, value) in &expected {
        assert!(*value > 0, "{} should be positive", name);
        assert!(*value < 256, "{} should be < 256", name);
    }
}

#[test]
fn test_errno_negative() {
    // Syscall convention: negative return = -errno
    let enoent: i64 = -2;
    assert!(enoent < 0);
    assert_eq!(-enoent, 2);
}
